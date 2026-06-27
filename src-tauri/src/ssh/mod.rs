//! SSH / SFTP 远程文件管理模块
//!
//! 第一版目标聚焦在“能稳定跑通”的文件管理主链路：
//! 1. 通过 SSH 用户名 + 密码连接远程虚拟机
//! 2. 通过 SFTP 列出远程目录
//! 3. 获取文件属性
//! 4. 下载远程文件到本地
//! 5. 对简单文本文件做本地缓存并在前端预览
//!
//! 设计说明：
//! - 连接层使用 `russh`
//! - 文件层使用 `russh-sftp`
//! - 会话层使用全局 `HashMap<session_id, session>`
//! - 路径层统一按 POSIX 风格处理远程路径，避免混入 Windows 路径语义

pub mod types;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Instant,
};

use anyhow::{anyhow, Context};
use russh::client;
use russh_sftp::{
    client::{fs::DirEntry, SftpSession},
    protocol::{FileType, OpenFlags},
};
use tauri::{AppHandle, Emitter, Manager, Window};
use time::format_description::well_known::Rfc3339;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use types::{
    CreateDirectoryResult, DeletePathResult, DownloadProgress, FileDownloadResult,
    FileUploadResult, OpenFileResult, RemoteFileEntry, RemoteFileProperties, RenamePathResult,
    SshCmdResult, SshConnectRequest, SshHostProbeRequest, SshHostProbeResult, SshSessionInfo,
    SuggestedDownloadPath, UploadProgress,
};
use uuid::Uuid;

/// 单个 SSH / SFTP 会话对象。
///
/// 这里仅保存业务所需的最小状态：
/// - `info`：给前端返回和展示的基础信息
/// - `sftp`：后续所有目录与文件操作都走它
///
/// 注意：
/// `SftpSession` 内部已经绑定到底层 SSH 通道，
/// 所以第一版无需额外单独保存 `russh::client::Handle`。
struct ManagedSshSession {
    /// SFTP 会话句柄。
    ///
    /// 使用 `Mutex` 的原因：
    /// 同一个会话的多个命令可能在极短时间内连续触发，
    /// 第一版优先保证串行化访问，避免底层通道并发使用带来的复杂问题。
    sftp: Mutex<SftpSession>,
}

/// 全局 SSH 会话表。
///
/// key：`session_id`
/// value：会话实例
static SSH_SESSION_MANAGER: OnceLock<Mutex<HashMap<String, Arc<ManagedSshSession>>>> =
    OnceLock::new();

/// 获取全局 SSH 会话管理器。
fn ssh_session_manager() -> &'static Mutex<HashMap<String, Arc<ManagedSshSession>>> {
    SSH_SESSION_MANAGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// SSH 客户端处理器。
///
/// 当前版本只实现最基础的服务端公钥校验逻辑：
/// - 允许连接继续建立
///
/// 这样可以最快交付第一版。
/// 后续建议升级为：
/// - 首次连接记录指纹（TOFU）
/// - 指纹变更时明确阻止并提示用户确认
#[derive(Clone)]
struct SshClientHandler {
    /// 期望匹配的主机指纹。
    ///
    /// 第二版要求前端在真正连接前先探测主机指纹，
    /// 用户确认后再把期望指纹传回这里执行严格校验。
    expected_fingerprint: Option<String>,
    /// 握手阶段实际观察到的主机指纹。
    ///
    /// 之所以要保存下来，是因为：
    /// - `ssh_probe_host` 需要把它返回给前端
    /// - `ssh_connect` 也需要把最终通过校验的指纹带回前端
    observed_fingerprint: Arc<Mutex<Option<String>>>,
}

impl Default for SshClientHandler {
    fn default() -> Self {
        Self {
            expected_fingerprint: None,
            observed_fingerprint: Arc::new(Mutex::new(None)),
        }
    }
}

impl client::Handler for SshClientHandler {
    type Error = anyhow::Error;

    /// 服务端主机密钥校验。
    ///
    /// 第一版为了先跑通功能，这里直接返回 `true`。
    /// 这意味着当前实现尚未完成主机指纹信任管理。
    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = compute_host_fingerprint(server_public_key);
        let mut observed = self.observed_fingerprint.lock().await;
        *observed = Some(fingerprint.clone());
        drop(observed);

        if let Some(expected) = &self.expected_fingerprint {
            Ok(expected == &fingerprint)
        } else {
            Ok(true)
        }
    }
}

/// 构造 SSH 主机校验处理器。
fn build_ssh_handler(expected_fingerprint: Option<String>) -> SshClientHandler {
    SshClientHandler {
        expected_fingerprint,
        observed_fingerprint: Arc::new(Mutex::new(None)),
    }
}

/// 从处理器中读取探测到的主机指纹。
async fn get_observed_fingerprint(handler: &SshClientHandler) -> anyhow::Result<String> {
    handler
        .observed_fingerprint
        .lock()
        .await
        .clone()
        .ok_or_else(|| anyhow!("未能获取远程主机指纹"))
}

/// 计算主机指纹。
///
/// 采用 `SHA256:...` 这种 OpenSSH 常见格式，
/// 便于用户与系统 `ssh`/`sftp` 输出互相对照。
fn compute_host_fingerprint(server_public_key: &russh::keys::ssh_key::PublicKey) -> String {
    server_public_key
        .fingerprint(russh::keys::ssh_key::HashAlg::Sha256)
        .to_string()
}

/// 建立 SSH 连接并创建 SFTP 会话。
#[tauri::command]
pub async fn ssh_connect(config: SshConnectRequest) -> SshCmdResult<SshSessionInfo> {
    match ssh_connect_inner(config).await {
        Ok(info) => SshCmdResult::ok(info),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 探测远程主机指纹。
///
/// 该命令只建立到 SSH 握手阶段的连接，不执行用户名密码认证。
/// 它的目的，是让前端能在正式连接之前先拿到主机指纹并让用户确认。
#[tauri::command]
pub async fn ssh_probe_host(config: SshHostProbeRequest) -> SshCmdResult<SshHostProbeResult> {
    match ssh_probe_host_inner(config).await {
        Ok(info) => SshCmdResult::ok(info),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 断开指定 SSH 会话。
#[tauri::command]
pub async fn ssh_disconnect(session_id: String) -> SshCmdResult<()> {
    match ssh_disconnect_inner(&session_id).await {
        Ok(()) => SshCmdResult::ok(()),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 列出远程目录中的文件与子目录。
#[tauri::command]
pub async fn sftp_list_directory(
    session_id: String,
    path: String,
) -> SshCmdResult<Vec<RemoteFileEntry>> {
    match sftp_list_directory_inner(&session_id, &path).await {
        Ok(entries) => SshCmdResult::ok(entries),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 获取单个远程文件或目录的详细属性。
#[tauri::command]
pub async fn sftp_get_properties(
    session_id: String,
    path: String,
) -> SshCmdResult<RemoteFileProperties> {
    match sftp_get_properties_inner(&session_id, &path).await {
        Ok(properties) => SshCmdResult::ok(properties),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 获取建议的本地下载路径。
///
/// 第一版没有引入系统原生“保存文件”对话框插件，
/// 因此这里先由后端根据系统下载目录给出默认值，
/// 再由前端允许用户继续修改。
#[tauri::command]
/// 第三版新增：在指定父目录下创建新的远程目录。
#[tauri::command]
/// 兼容旧注释格式的占位命令。
///
/// 这个空命令本身不会被前端调用，它的唯一作用是“吸收”
/// 上一版遗留下来的属性宏，避免真正的 `sftp_create_directory`
/// 因为重复 `#[tauri::command]` 而编译失败。
#[allow(dead_code)]
fn __ssh_legacy_comment_barrier_create_directory() {}

pub async fn sftp_create_directory(
    session_id: String,
    parent_path: String,
    directory_name: String,
) -> SshCmdResult<CreateDirectoryResult> {
    match sftp_create_directory_inner(&session_id, &parent_path, &directory_name).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 第三版新增：重命名远程文件或目录。
///
/// 当前版本只改“名称”，不支持跨目录移动，
/// 这样可以把交互控制得更直接、更安全。
#[tauri::command]
pub async fn sftp_rename_path(
    session_id: String,
    source_path: String,
    new_name: String,
) -> SshCmdResult<RenamePathResult> {
    match sftp_rename_path_inner(&session_id, &source_path, &new_name).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 第三版新增：删除远程文件或目录。
///
/// 当前版本支持删除普通文件和空目录。
#[tauri::command]
pub async fn sftp_delete_path(
    session_id: String,
    path: String,
) -> SshCmdResult<DeletePathResult> {
    match sftp_delete_path_inner(&session_id, &path).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

#[tauri::command]
pub async fn sftp_suggest_download_path(
    app: AppHandle,
    file_name: String,
) -> SshCmdResult<SuggestedDownloadPath> {
    match suggest_download_path_inner(&app, &file_name) {
        Ok(suggested_path) => SshCmdResult::ok(SuggestedDownloadPath { suggested_path }),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 下载远程文件到用户指定的本地路径。
#[tauri::command]
pub async fn sftp_download_file(
    window: Window,
    session_id: String,
    remote_path: String,
    local_path: String,
) -> SshCmdResult<FileDownloadResult> {
    match sftp_download_file_inner(&window, &session_id, &remote_path, &local_path).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 把本地文件上传到当前远程目录。
///
/// 第二版中，前端会先通过原生文件选择器让用户选本地文件，
/// 再把本地路径和当前远程目录传给这里。
#[tauri::command]
pub async fn sftp_upload_file(
    window: Window,
    session_id: String,
    local_path: String,
    remote_dir: String,
) -> SshCmdResult<FileUploadResult> {
    match sftp_upload_file_inner(Some(&window), &session_id, &local_path, &remote_dir).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 提供给示例程序的无事件上传接口。
///
/// 它不会向前端发送进度事件，仅用于命令行验证第二版上传逻辑。
pub async fn sftp_upload_file_without_events(
    session_id: String,
    local_path: String,
    remote_dir: String,
) -> SshCmdResult<FileUploadResult> {
    match sftp_upload_file_inner(None, &session_id, &local_path, &remote_dir).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// 本地打开简单文本文件。
///
/// 这里的“打开”实现为：
/// - 下载远程文件到本地临时缓存目录
/// - 如果该文件属于常见文本类型，则直接把文本内容返回前端预览
#[tauri::command]
pub async fn sftp_open_text_file(
    app: AppHandle,
    session_id: String,
    remote_path: String,
) -> SshCmdResult<OpenFileResult> {
    match sftp_open_text_file_inner(&app, &session_id, &remote_path).await {
        Ok(result) => SshCmdResult::ok(result),
        Err(error) => SshCmdResult::err(error.to_string()),
    }
}

/// SSH 连接主流程。
async fn ssh_connect_inner(config: SshConnectRequest) -> anyhow::Result<SshSessionInfo> {
    let host = config.host.trim();
    if host.is_empty() {
        return Err(anyhow!("远程主机地址不能为空"));
    }

    let username = config.username.trim();
    if username.is_empty() {
        return Err(anyhow!("登录用户名不能为空"));
    }

    let expected_fingerprint = config
        .expected_host_fingerprint
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("缺少主机指纹确认信息，请先完成主机指纹确认"))?;

    let mut ssh_config = russh::client::Config::default();
    ssh_config.inactivity_timeout = Some(std::time::Duration::from_secs(60));
    let ssh_config = Arc::new(ssh_config);

    let address = format!("{}:{}", host, config.port);
    let handler = build_ssh_handler(Some(expected_fingerprint.clone()));
    let mut handle = client::connect(ssh_config, address, handler.clone())
        .await
        .with_context(|| format!("SSH 连接失败，或主机指纹与预期不一致：{}", expected_fingerprint))?;

    let observed_fingerprint = get_observed_fingerprint(&handler).await?;

    let auth_result = handle
        .authenticate_password(username, config.password)
        .await
        .context("SSH 用户名或密码认证失败")?;

    if !auth_result.success() {
        return Err(anyhow!("SSH 认证失败，请检查用户名或密码"));
    }

    let channel = handle
        .channel_open_session()
        .await
        .context("无法打开 SSH 会话通道")?;

    channel
        .request_subsystem(true, "sftp")
        .await
        .context("无法启动 SFTP 子系统")?;

    let sftp = SftpSession::new(channel.into_stream())
        .await
        .context("SFTP 会话初始化失败")?;

    let initial_path = normalize_remote_path(config.initial_path.as_deref().unwrap_or("/"));
    let current_path = match sftp.canonicalize(initial_path.as_str()).await {
        Ok(path) => normalize_remote_path(&path),
        Err(_) => "/".to_string(),
    };

    let session_id = Uuid::new_v4().to_string();
    let session_info = SshSessionInfo {
        session_id: session_id.clone(),
        host: host.to_string(),
        port: config.port,
        username: username.to_string(),
        current_path,
        host_fingerprint: observed_fingerprint,
    };

    let session = Arc::new(ManagedSshSession {
        sftp: Mutex::new(sftp),
    });

    let mut manager = ssh_session_manager().lock().await;
    manager.insert(session_id, session);

    Ok(session_info)
}

/// SSH 主机指纹探测流程。
async fn ssh_probe_host_inner(config: SshHostProbeRequest) -> anyhow::Result<SshHostProbeResult> {
    let host = config.host.trim();
    if host.is_empty() {
        return Err(anyhow!("远程主机地址不能为空"));
    }

    let mut ssh_config = russh::client::Config::default();
    ssh_config.inactivity_timeout = Some(std::time::Duration::from_secs(30));
    let ssh_config = Arc::new(ssh_config);

    let handler = build_ssh_handler(None);
    let address = format!("{}:{}", host, config.port);

    let handle = client::connect(ssh_config, address, handler.clone())
        .await
        .context("SSH 主机指纹探测失败")?;

    drop(handle);

    Ok(SshHostProbeResult {
        host: host.to_string(),
        port: config.port,
        fingerprint: get_observed_fingerprint(&handler).await?,
    })
}

/// 断开连接并从全局会话表中移除。
async fn ssh_disconnect_inner(session_id: &str) -> anyhow::Result<()> {
    let session = {
        let mut manager = ssh_session_manager().lock().await;
        manager.remove(session_id)
    };

    if let Some(session) = session {
        let sftp = session.sftp.lock().await;
        let _ = sftp.close().await;
    }

    Ok(())
}

/// 列目录内部实现。
async fn sftp_list_directory_inner(
    session_id: &str,
    path: &str,
) -> anyhow::Result<Vec<RemoteFileEntry>> {
    let session = get_session(session_id).await?;
    let target_path = normalize_remote_path(path);

    let sftp = session.sftp.lock().await;
    let canonical_path = match sftp.canonicalize(target_path.as_str()).await {
        Ok(pathbuf) => normalize_remote_path(&pathbuf),
        Err(_) => target_path.clone(),
    };

    let entries = sftp
        .read_dir(canonical_path.as_str())
        .await
        .with_context(|| format!("读取远程目录失败：{}", canonical_path))?;

    let mut result = Vec::new();
    for entry in entries {
        if let Some(file_entry) = convert_dir_entry(&canonical_path, entry)? {
            result.push(file_entry);
        }
    }

    result.sort_by(|left, right| {
        match (left.is_dir, right.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        }
    });

    Ok(result)
}

/// 获取文件属性内部实现。
async fn sftp_get_properties_inner(
    session_id: &str,
    path: &str,
) -> anyhow::Result<RemoteFileProperties> {
    let session = get_session(session_id).await?;
    let target_path = normalize_remote_path(path);

    let sftp = session.sftp.lock().await;
    let metadata = sftp
        .symlink_metadata(target_path.as_str())
        .await
        .with_context(|| format!("读取远程文件属性失败：{}", target_path))?;

    Ok(build_properties_from_metadata(&target_path, &metadata))
}

/// 生成建议下载路径。
/// 第三版新增：创建远程目录内部实现。
async fn sftp_create_directory_inner(
    session_id: &str,
    parent_path: &str,
    directory_name: &str,
) -> anyhow::Result<CreateDirectoryResult> {
    let session = get_session(session_id).await?;
    let normalized_parent = normalize_remote_path(parent_path);
    let validated_name = validate_remote_entry_name(directory_name)?;
    let target_path = join_remote_path(&normalized_parent, &validated_name);

    let sftp = session.sftp.lock().await;
    sftp.create_dir(target_path.as_str())
        .await
        .with_context(|| format!("创建远程目录失败：{}", target_path))?;

    Ok(CreateDirectoryResult { path: target_path })
}

/// 第三版新增：重命名远程路径内部实现。
async fn sftp_rename_path_inner(
    session_id: &str,
    source_path: &str,
    new_name: &str,
) -> anyhow::Result<RenamePathResult> {
    let session = get_session(session_id).await?;
    let normalized_source = normalize_remote_path(source_path);
    let validated_name = validate_remote_entry_name(new_name)?;

    if normalized_source == "/" {
        return Err(anyhow!("根目录不允许重命名"));
    }

    let parent_path = get_parent_remote_path(&normalized_source);
    let target_path = join_remote_path(&parent_path, &validated_name);

    let sftp = session.sftp.lock().await;
    sftp.rename(normalized_source.as_str(), target_path.as_str())
        .await
        .with_context(|| format!("重命名远程路径失败：{} -> {}", normalized_source, target_path))?;

    Ok(RenamePathResult {
        old_path: normalized_source,
        new_path: target_path,
    })
}

/// 第三版新增：删除远程路径内部实现。
///
/// 为了降低误删风险，目录删除只允许删除空目录。
async fn sftp_delete_path_inner(
    session_id: &str,
    path: &str,
) -> anyhow::Result<DeletePathResult> {
    let session = get_session(session_id).await?;
    let target_path = normalize_remote_path(path);

    if target_path == "/" {
        return Err(anyhow!("根目录不允许删除"));
    }

    let sftp = session.sftp.lock().await;
    let metadata = sftp
        .symlink_metadata(target_path.as_str())
        .await
        .with_context(|| format!("读取远程路径属性失败：{}", target_path))?;

    let is_dir = matches!(metadata.file_type(), FileType::Dir);

    if is_dir {
        let entries = sftp
            .read_dir(target_path.as_str())
            .await
            .with_context(|| format!("读取远程目录失败：{}", target_path))?;

        let has_child = entries
            .into_iter()
            .any(|entry| !matches!(entry.file_name().as_str(), "." | ".."));

        if has_child {
            return Err(anyhow!("当前版本仅支持删除空目录，请先清空目录内容"));
        }

        sftp.remove_dir(target_path.as_str())
            .await
            .with_context(|| format!("删除远程目录失败：{}", target_path))?;
    } else {
        sftp.remove_file(target_path.as_str())
            .await
            .with_context(|| format!("删除远程文件失败：{}", target_path))?;
    }

    Ok(DeletePathResult {
        path: target_path,
        is_dir,
    })
}

fn suggest_download_path_inner(app: &AppHandle, file_name: &str) -> anyhow::Result<String> {
    let download_dir = match app.path().download_dir() {
        Ok(path) => path,
        Err(_) => std::env::temp_dir(),
    };

    Ok(download_dir.join(file_name).to_string_lossy().to_string())
}

/// 下载文件内部实现。
async fn sftp_download_file_inner(
    window: &Window,
    session_id: &str,
    remote_path: &str,
    local_path: &str,
) -> anyhow::Result<FileDownloadResult> {
    let session = get_session(session_id).await?;
    let remote_path = normalize_remote_path(remote_path);
    let local_path = PathBuf::from(local_path);

    if local_path.as_os_str().is_empty() {
        return Err(anyhow!("本地保存路径不能为空"));
    }

    let start = Instant::now();
    let file_size = {
        let sftp = session.sftp.lock().await;
        let metadata = sftp
            .metadata(remote_path.as_str())
            .await
            .with_context(|| format!("读取远程文件大小失败：{}", remote_path))?;
        metadata.len()
    };

    emit_progress(
        window,
        DownloadProgress {
            session_id: session_id.to_string(),
            remote_path: remote_path.clone(),
            local_path: local_path.to_string_lossy().to_string(),
            downloaded_bytes: 0,
            total_bytes: file_size,
            progress: 0.0,
            stage: "checking".to_string(),
            message: "正在检查远程文件".to_string(),
        },
    );

    if let Some(parent_dir) = local_path.parent() {
        fs::create_dir_all(parent_dir)
            .await
            .with_context(|| format!("无法创建本地目录：{}", parent_dir.display()))?;
    }

    let temp_path = local_path.with_extension("part");
    let result = download_remote_file_to_local(
        &session,
        session_id,
        &remote_path,
        &local_path,
        &temp_path,
        Some(window),
    )
    .await?;

    Ok(FileDownloadResult {
        remote_path,
        local_path: result.to_string_lossy().to_string(),
        file_size,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// 上传文件内部实现。
async fn sftp_upload_file_inner(
    window: Option<&Window>,
    session_id: &str,
    local_path: &str,
    remote_dir: &str,
) -> anyhow::Result<FileUploadResult> {
    let session = get_session(session_id).await?;
    let local_path = PathBuf::from(local_path);
    if local_path.as_os_str().is_empty() {
        return Err(anyhow!("本地文件路径不能为空"));
    }

    let file_name = local_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("无法识别本地文件名"))?;
    let remote_dir = normalize_remote_path(remote_dir);
    let remote_path = join_remote_path(&remote_dir, file_name);

    let metadata = fs::metadata(&local_path)
        .await
        .with_context(|| format!("无法读取本地文件属性：{}", local_path.display()))?;

    if !metadata.is_file() {
        return Err(anyhow!("当前版本仅支持上传普通文件"));
    }

    let total_bytes = metadata.len();
    let start = Instant::now();

    if let Some(window) = window {
        emit_upload_progress(
            window,
            UploadProgress {
                session_id: session_id.to_string(),
                local_path: local_path.to_string_lossy().to_string(),
                remote_path: remote_path.clone(),
                uploaded_bytes: 0,
                total_bytes,
                progress: 0.0,
                stage: "checking".to_string(),
                message: "正在检查本地文件".to_string(),
            },
        );
    }

    upload_local_file_to_remote(
        &session,
        session_id,
        &local_path,
        &remote_path,
        window,
    )
    .await?;

    Ok(FileUploadResult {
        local_path: local_path.to_string_lossy().to_string(),
        remote_path,
        file_size: total_bytes,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// 本地打开文本文件内部实现。
async fn sftp_open_text_file_inner(
    app: &AppHandle,
    session_id: &str,
    remote_path: &str,
) -> anyhow::Result<OpenFileResult> {
    let session = get_session(session_id).await?;
    let remote_path = normalize_remote_path(remote_path);
    let cache_path = build_cache_file_path(app, session_id, &remote_path);

    if let Some(parent_dir) = cache_path.parent() {
        fs::create_dir_all(parent_dir)
            .await
            .with_context(|| format!("无法创建缓存目录：{}", parent_dir.display()))?;
    }

    let temp_path = cache_path.with_extension("tmp");
    let saved_path = download_remote_file_to_local(
        &session,
        session_id,
        &remote_path,
        &cache_path,
        &temp_path,
        None,
    )
    .await?;

    let metadata = fs::metadata(&saved_path)
        .await
        .with_context(|| format!("无法读取缓存文件属性：{}", saved_path.display()))?;
    let file_size = metadata.len();

    let is_text = is_text_file(&remote_path);
    if !is_text {
        return Err(anyhow!("当前第一版仅支持文本类文件的本地打开"));
    }

    // 这里给文本预览设置一个相对保守的上限，避免大文件直接塞进前端导致卡顿。
    const MAX_TEXT_PREVIEW_BYTES: u64 = 1024 * 1024;
    if file_size > MAX_TEXT_PREVIEW_BYTES {
        return Err(anyhow!(
            "文本文件超过 1 MB，第一版暂不支持直接预览，请先下载到本地查看"
        ));
    }

    let content = fs::read(&saved_path)
        .await
        .with_context(|| format!("无法读取缓存文件内容：{}", saved_path.display()))?;

    Ok(OpenFileResult {
        remote_path,
        local_path: saved_path.to_string_lossy().to_string(),
        file_size,
        is_text: true,
        text_content: Some(String::from_utf8_lossy(&content).to_string()),
    })
}

/// 从全局会话表中获取指定会话。
async fn get_session(session_id: &str) -> anyhow::Result<Arc<ManagedSshSession>> {
    let manager = ssh_session_manager().lock().await;
    manager
        .get(session_id)
        .cloned()
        .ok_or_else(|| anyhow!("SSH 会话不存在或已断开，请重新连接"))
}

/// 把远程文件下载到本地。
///
/// 这是下载命令与文本预览命令共用的底层能力。
async fn download_remote_file_to_local(
    session: &ManagedSshSession,
    session_id: &str,
    remote_path: &str,
    final_local_path: &Path,
    temp_local_path: &Path,
    window: Option<&Window>,
) -> anyhow::Result<PathBuf> {
    let sftp = session.sftp.lock().await;
    let metadata = sftp
        .metadata(remote_path)
        .await
        .with_context(|| format!("读取远程文件属性失败：{}", remote_path))?;

    if metadata.is_dir() {
        return Err(anyhow!("当前版本暂不支持下载目录"));
    }

    let total_bytes = metadata.len();
    let mut remote_file = sftp
        .open(remote_path)
        .await
        .with_context(|| format!("无法打开远程文件：{}", remote_path))?;
    drop(sftp);

    let mut local_file = fs::File::create(temp_local_path)
        .await
        .with_context(|| format!("无法创建本地文件：{}", temp_local_path.display()))?;

    let mut buffer = vec![0_u8; 64 * 1024];
    let mut downloaded_bytes = 0_u64;

    loop {
        let read_size = remote_file
            .read(&mut buffer)
            .await
            .with_context(|| format!("读取远程文件失败：{}", remote_path))?;

        if read_size == 0 {
            break;
        }

        local_file
            .write_all(&buffer[..read_size])
            .await
            .with_context(|| format!("写入本地文件失败：{}", temp_local_path.display()))?;

        downloaded_bytes += read_size as u64;

        if let Some(window) = window {
            emit_progress(
                window,
                DownloadProgress {
                    session_id: session_id.to_string(),
                    remote_path: remote_path.to_string(),
                    local_path: final_local_path.to_string_lossy().to_string(),
                    downloaded_bytes,
                    total_bytes,
                    progress: calculate_progress(downloaded_bytes, total_bytes),
                    stage: "downloading".to_string(),
                    message: format!(
                        "正在下载：{} / {}",
                        format_size(downloaded_bytes),
                        format_size(total_bytes)
                    ),
                },
            );
        }
    }

    local_file
        .flush()
        .await
        .with_context(|| format!("刷新本地文件失败：{}", temp_local_path.display()))?;

    if let Some(window) = window {
        emit_progress(
            window,
            DownloadProgress {
                session_id: session_id.to_string(),
                remote_path: remote_path.to_string(),
                local_path: final_local_path.to_string_lossy().to_string(),
                downloaded_bytes,
                total_bytes,
                progress: 1.0,
                stage: "saving".to_string(),
                message: "正在完成本地保存".to_string(),
            },
        );
    }

    fs::rename(temp_local_path, final_local_path)
        .await
        .with_context(|| format!("无法完成本地文件保存：{}", final_local_path.display()))?;

    if let Some(window) = window {
        emit_progress(
            window,
            DownloadProgress {
                session_id: session_id.to_string(),
                remote_path: remote_path.to_string(),
                local_path: final_local_path.to_string_lossy().to_string(),
                downloaded_bytes,
                total_bytes,
                progress: 1.0,
                stage: "completed".to_string(),
                message: "下载完成".to_string(),
            },
        );
    }

    Ok(final_local_path.to_path_buf())
}

/// 把本地文件上传到远程目标路径。
async fn upload_local_file_to_remote(
    session: &ManagedSshSession,
    session_id: &str,
    local_path: &Path,
    remote_path: &str,
    window: Option<&Window>,
) -> anyhow::Result<()> {
    let mut local_file = fs::File::open(local_path)
        .await
        .with_context(|| format!("无法打开本地文件：{}", local_path.display()))?;
    let total_bytes = local_file
        .metadata()
        .await
        .with_context(|| format!("无法读取本地文件属性：{}", local_path.display()))?
        .len();

    let sftp = session.sftp.lock().await;
    let mut remote_file = sftp
        .open_with_flags(
            remote_path,
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await
        .with_context(|| format!("无法创建远程文件：{}", remote_path))?;
    drop(sftp);

    let mut buffer = vec![0_u8; 64 * 1024];
    let mut uploaded_bytes = 0_u64;

    loop {
        let read_size = local_file
            .read(&mut buffer)
            .await
            .with_context(|| format!("读取本地文件失败：{}", local_path.display()))?;

        if read_size == 0 {
            break;
        }

        remote_file
            .write_all(&buffer[..read_size])
            .await
            .with_context(|| format!("写入远程文件失败：{}", remote_path))?;

        uploaded_bytes += read_size as u64;

        if let Some(window) = window {
            emit_upload_progress(
                window,
                UploadProgress {
                    session_id: session_id.to_string(),
                    local_path: local_path.to_string_lossy().to_string(),
                    remote_path: remote_path.to_string(),
                    uploaded_bytes,
                    total_bytes,
                    progress: calculate_progress(uploaded_bytes, total_bytes),
                    stage: "uploading".to_string(),
                    message: format!(
                        "正在上传：{} / {}",
                        format_size(uploaded_bytes),
                        format_size(total_bytes)
                    ),
                },
            );
        }
    }

    remote_file
        .flush()
        .await
        .with_context(|| format!("刷新远程文件失败：{}", remote_path))?;
    remote_file
        .shutdown()
        .await
        .with_context(|| format!("完成远程文件保存失败：{}", remote_path))?;

    if let Some(window) = window {
        emit_upload_progress(
            window,
            UploadProgress {
                session_id: session_id.to_string(),
                local_path: local_path.to_string_lossy().to_string(),
                remote_path: remote_path.to_string(),
                uploaded_bytes,
                total_bytes,
                progress: 1.0,
                stage: "completed".to_string(),
                message: "上传完成".to_string(),
            },
        );
    }

    Ok(())
}

/// 将目录项转换为前端表格结构。
fn convert_dir_entry(
    parent_path: &str,
    entry: DirEntry,
) -> anyhow::Result<Option<RemoteFileEntry>> {
    let file_name = entry.file_name();
    if file_name == "." || file_name == ".." {
        return Ok(None);
    }

    let metadata = entry.metadata();
    let file_path = join_remote_path(parent_path, &file_name);
    let kind = parse_file_kind(metadata.file_type());

    Ok(Some(RemoteFileEntry {
        name: file_name,
        path: file_path,
        is_dir: kind.is_dir,
        is_file: kind.is_file,
        is_symlink: kind.is_symlink,
        file_type: kind.file_type,
        size: metadata.len(),
        permissions: metadata.permissions,
        permission_text: format_permission_text(metadata.permissions),
        uid: metadata.uid,
        gid: metadata.gid,
        modified_at: format_system_time_result(metadata.modified()),
        accessed_at: format_system_time_result(metadata.accessed()),
    }))
}

/// 根据 metadata 构造文件属性结构。
fn build_properties_from_metadata(
    path: &str,
    metadata: &russh_sftp::client::fs::Metadata,
) -> RemoteFileProperties {
    let file_name = path
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string();
    let kind = parse_file_kind(metadata.file_type());

    RemoteFileProperties {
        name: file_name,
        path: path.to_string(),
        is_dir: kind.is_dir,
        is_file: kind.is_file,
        is_symlink: kind.is_symlink,
        file_type: kind.file_type,
        size: metadata.len(),
        permissions: metadata.permissions,
        permission_text: format_permission_text(metadata.permissions),
        uid: metadata.uid,
        gid: metadata.gid,
        modified_at: format_system_time_result(metadata.modified()),
        accessed_at: format_system_time_result(metadata.accessed()),
    }
}

/// 远程文件类型解析结果。
struct FileKindInfo {
    is_dir: bool,
    is_file: bool,
    is_symlink: bool,
    file_type: String,
}

/// 解析 POSIX 权限位中的文件类型。
fn parse_file_kind(file_type: FileType) -> FileKindInfo {
    match file_type {
        FileType::Dir => FileKindInfo {
            is_dir: true,
            is_file: false,
            is_symlink: false,
            file_type: "directory".to_string(),
        },
        FileType::Symlink => FileKindInfo {
            is_dir: false,
            is_file: false,
            is_symlink: true,
            file_type: "symlink".to_string(),
        },
        FileType::File => FileKindInfo {
            is_dir: false,
            is_file: true,
            is_symlink: false,
            file_type: "file".to_string(),
        },
        FileType::Other => FileKindInfo {
            is_dir: false,
            is_file: false,
            is_symlink: false,
            file_type: "unknown".to_string(),
        },
    }
}

/// 格式化权限文本。
///
/// 只取低 9 位，显示为常见的八进制权限字符串，如：`755`、`644`。
fn format_permission_text(permissions: Option<u32>) -> Option<String> {
    permissions.map(|perm| format!("{:03o}", perm & 0o777))
}

/// 把 Unix 时间戳格式化为 ISO 8601 字符串。
fn format_system_time_result(
    timestamp: std::io::Result<std::time::SystemTime>,
) -> Option<String> {
    timestamp.ok().and_then(|value| {
        let unix_time = value
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs() as i64)?;

        time::OffsetDateTime::from_unix_timestamp(unix_time)
            .ok()
            .and_then(|datetime| datetime.format(&Rfc3339).ok())
    })
}

/// 计算下载进度。
fn calculate_progress(downloaded_bytes: u64, total_bytes: u64) -> f64 {
    if total_bytes == 0 {
        return 0.0;
    }
    downloaded_bytes as f64 / total_bytes as f64
}

/// 以人类友好的方式格式化字节大小。
fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size_f64 = size as f64;
    if size_f64 >= GB {
        format!("{:.2} GB", size_f64 / GB)
    } else if size_f64 >= MB {
        format!("{:.2} MB", size_f64 / MB)
    } else if size_f64 >= KB {
        format!("{:.2} KB", size_f64 / KB)
    } else {
        format!("{} B", size)
    }
}

/// 触发前端下载进度事件。
fn emit_progress(window: &Window, progress: DownloadProgress) {
    let _ = window.emit("ssh-download-progress", progress);
}

/// 触发前端上传进度事件。
fn emit_upload_progress(window: &Window, progress: UploadProgress) {
    let _ = window.emit("ssh-upload-progress", progress);
}

/// 构造文本预览缓存路径。
fn build_cache_file_path(app: &AppHandle, session_id: &str, remote_path: &str) -> PathBuf {
    let mut base_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("learn_tauri"));

    base_dir.push("remote_file_cache");
    base_dir.push(session_id);

    for segment in remote_path.split('/').filter(|segment| !segment.is_empty()) {
        base_dir.push(segment);
    }

    if remote_path.ends_with('/') || remote_path == "/" {
        base_dir.push("index.txt");
    }

    base_dir
}

/// 判断远程文件是否属于常见文本类型。
fn is_text_file(path: &str) -> bool {
    let extension = path
        .rsplit('.')
        .next()
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    matches!(
        extension.as_str(),
        "txt"
            | "log"
            | "conf"
            | "cfg"
            | "ini"
            | "json"
            | "yaml"
            | "yml"
            | "xml"
            | "toml"
            | "md"
            | "sh"
            | "py"
            | "rs"
            | "ts"
            | "js"
            | "vue"
            | "java"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "csv"
    )
}

/// 统一规范远程路径格式。
///
/// 规则：
/// - 空路径 => `/`
/// - 反斜杠 => 正斜杠
/// - 多个连续 `/` 合并为一个
/// 校验远程文件或目录名称。
///
/// 这里故意只允许传入“单段名称”，不允许包含 `/` 或 `\`，
/// 避免前端借由重命名或新建目录操作变相跨目录写入。
fn validate_remote_entry_name(name: &str) -> anyhow::Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("名称不能为空"));
    }

    if trimmed == "." || trimmed == ".." {
        return Err(anyhow!("名称不能为 . 或 .."));
    }

    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(anyhow!("名称中不能包含路径分隔符"));
    }

    Ok(trimmed.to_string())
}

/// 计算某个远程路径的父目录。
fn get_parent_remote_path(path: &str) -> String {
    let normalized = normalize_remote_path(path);
    if normalized == "/" {
        return "/".to_string();
    }

    let mut segments = normalized.split('/').filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
    segments.pop();

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn normalize_remote_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }

    let replaced = trimmed.replace('\\', "/");
    let mut normalized = String::with_capacity(replaced.len());
    let mut last_was_slash = false;

    for ch in replaced.chars() {
        if ch == '/' {
            if !last_was_slash {
                normalized.push(ch);
            }
            last_was_slash = true;
        } else {
            normalized.push(ch);
            last_was_slash = false;
        }
    }

    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}

/// 拼接父目录与子文件名，始终输出 POSIX 风格路径。
fn join_remote_path(parent: &str, file_name: &str) -> String {
    if parent == "/" {
        format!("/{}", file_name)
    } else {
        format!("{}/{}", parent.trim_end_matches('/'), file_name)
    }
}
