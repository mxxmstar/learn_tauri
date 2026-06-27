//! Telnet 客户端核心实现
//!
//! 实现 TelnetClient 结构体，封装 TCP 连接、登录认证、命令执行和文件下载功能。
//! 使用简单 TCP 文本命令方式（类似串口通信），不实现完整的 telnet 协议选项协商。

use crate::telnet::config::TelnetConfig;
use crate::telnet::error::{TelnetError, TelnetResult};
use crate::telnet::types::{ConnectionStatus, DownloadProgress};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

/// Telnet 客户端
///
/// 提供面向文本的 TCP 通信接口，支持：
/// - 连接管理（建立/断开连接）
/// - 登录认证（自动检测 login/password 提示符）
/// - 命令执行（发送命令并读取输出）
/// - 文件下载（通过 cat 命令读取文件内容）
///
/// # 线程安全
///
/// 内部使用 `Arc<Mutex>` 共享状态，可以安全地克隆并在多个异步任务间使用。
#[derive(Clone)]
pub struct TelnetClient {
    /// 客户端配置
    config: Arc<TelnetConfig>,
    /// TCP 连接流（连接后存在）
    stream: Arc<Mutex<Option<TcpStream>>>,
    /// 连接状态
    status: Arc<Mutex<ConnectionStatus>>,
}

impl TelnetClient {
    /// 创建新的 Telnet 客户端实例
    ///
    /// # 参数
    /// * `config` - telnet 配置
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 TelnetClient 实例
    pub fn new(config: TelnetConfig) -> TelnetResult<Self> {
        // 验证配置有效性
        config.validate()
            .map_err(|e| TelnetError::ConfigError(e))?;

        Ok(Self {
            config: Arc::new(config),
            stream: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
        })
    }

    /// 连接到设备
    ///
    /// 建立 TCP 连接，受 `connect_timeout_ms` 控制。
    ///
    /// # 返回值
    /// 返回 Result，成功时返回 ()
    pub async fn connect(&self) -> TelnetResult<()> {
        let mut status = self.status.lock().await;
        *status = ConnectionStatus::Connecting;
        drop(status);

        let addr = self.config.parse_addr()
            .map_err(|e| TelnetError::ConfigError(e))?;

        // 使用超时包装连接操作
        let stream = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.connect_timeout_ms),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| {
            TelnetError::ConnectionTimeout(format!(
                "连接超时（{}ms）: {}",
                self.config.connect_timeout_ms, addr
            ))
        })?
        .map_err(|e| TelnetError::ConnectionError(format!("连接失败: {}", e)))?;

        // 设置 TCP 选项
        if let Err(e) = stream.set_nodelay(true) {
            eprintln!("设置 TCP_NODELAY 失败: {}", e);
        }

        // 保存连接
        *self.stream.lock().await = Some(stream);
        *self.status.lock().await = ConnectionStatus::Connected;

        Ok(())
    }

    /// 登录设备
    ///
    /// 自动检测 login:、Password: 提示符，发送用户名和密码。
    ///
    /// # 参数
    /// * `username` - 登录用户名
    /// * `password` - 登录密码
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 LoginResult
    pub async fn login(&self, username: &str, password: &str) -> TelnetResult<crate::telnet::types::LoginResult> {
        use crate::telnet::types::LoginResult;

        // 检查连接状态
        if !self.is_connected().await {
            return Err(TelnetError::NotConnected("请先连接设备".to_string()));
        }

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| TelnetError::NotConnected("连接已断开".to_string()))?;

        let mut output = String::new();
        let start_time = Instant::now();

        // 读取初始输出，等待 login 提示符
        let mut buf = [0u8; 1024];

        // 等待 login: 提示符
        loop {
            if start_time.elapsed().as_millis() as u64 > self.config.login_timeout_ms {
                return Err(TelnetError::LoginTimeout("等待登录提示符超时".to_string()));
            }

            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                stream.read(&mut buf),
            )
            .await
            {
                Ok(Ok(0)) => {
                    // 连接关闭
                    return Err(TelnetError::NotConnected("连接已关闭".to_string()));
                }
                Ok(Ok(n)) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    output.push_str(&chunk);

                    // 检测 login 提示符
                    if output.to_lowercase().contains(&self.config.login_prompt.to_lowercase()) {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(TelnetError::IoError(format!("读取数据失败: {}", e)));
                }
                Err(_) => {
                    // 超时，继续等待
                    continue;
                }
            }
        }

        // 发送用户名
        stream.write_all(username.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        output.push_str(username);
        output.push('\n');

        // 等待 password 提示符
        loop {
            if start_time.elapsed().as_millis() as u64 > self.config.login_timeout_ms {
                return Err(TelnetError::LoginTimeout("等待密码提示符超时".to_string()));
            }

            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                stream.read(&mut buf),
            )
            .await
            {
                Ok(Ok(0)) => {
                    return Err(TelnetError::NotConnected("连接已关闭".to_string()));
                }
                Ok(Ok(n)) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    output.push_str(&chunk);

                    // 检测 password 提示符
                    if output.to_lowercase().contains(&self.config.password_prompt.to_lowercase()) {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(TelnetError::IoError(format!("读取数据失败: {}", e)));
                }
                Err(_) => {
                    // 超时，继续等待
                    continue;
                }
            }
        }

        // 发送密码
        stream.write_all(password.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        output.push_str("***"); // 密码不记录在输出中
        output.push('\n');

        // 等待 shell 提示符（登录成功）
        let mut prompt = String::new();
        loop {
            if start_time.elapsed().as_millis() as u64 > self.config.login_timeout_ms {
                return Err(TelnetError::LoginTimeout("等待 shell 提示符超时".to_string()));
            }

            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                stream.read(&mut buf),
            )
            .await
            {
                Ok(Ok(0)) => {
                    return Err(TelnetError::NotConnected("连接已关闭".to_string()));
                }
                Ok(Ok(n)) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    output.push_str(&chunk);

                    // 检测 shell 提示符
                    for p in &self.config.shell_prompts {
                        if output.contains(p) {
                            prompt = p.clone();
                            break;
                        }
                    }

                    if !prompt.is_empty() {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(TelnetError::IoError(format!("读取数据失败: {}", e)));
                }
                Err(_) => {
                    // 超时，继续等待
                    continue;
                }
            }
        }

        // 更新状态
        *self.status.lock().await = ConnectionStatus::LoggedIn;

        // 清理 ANSI 转义序列
        let clean_output = if self.config.clean_ansi {
            clean_ansi_sequences(&output)
        } else {
            output.clone()
        };

        Ok(LoginResult::success(&prompt, &clean_output))
    }

    /// 执行命令
    ///
    /// 发送命令并读取输出，直到检测到 shell 提示符。
    ///
    /// # 参数
    /// * `command` - 要执行的命令
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 CommandResult
    pub async fn execute_command(
        &self,
        command: &str,
    ) -> TelnetResult<crate::telnet::types::CommandResult> {
        use crate::telnet::types::CommandResult;

        // 检查连接状态
        if !self.is_logged_in().await {
            return Err(TelnetError::NotConnected(
                "请先连接并登录设备".to_string(),
            ));
        }

        let mut stream_guard = self.stream.lock().await;
        let stream = stream_guard
            .as_mut()
            .ok_or_else(|| TelnetError::NotConnected("连接已断开".to_string()))?;

        let start_time = Instant::now();
        let mut output = String::new();

        // 发送命令
        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        // 读取输出，直到检测到 shell 提示符
        let mut buf = [0u8; 4096];
        loop {
            if start_time.elapsed().as_millis() as u64 > self.config.command_timeout_ms {
                return Err(TelnetError::CommandTimeout(format!(
                    "命令执行超时（{}ms）: {}",
                    self.config.command_timeout_ms, command
                )));
            }

            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                stream.read(&mut buf),
            )
            .await
            {
                Ok(Ok(0)) => {
                    // 连接关闭
                    return Err(TelnetError::NotConnected("连接已关闭".to_string()));
                }
                Ok(Ok(n)) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    output.push_str(&chunk);

                    // 检测 shell 提示符
                    let mut prompt_detected = false;
                    for p in &self.config.shell_prompts {
                        if output.contains(p) {
                            prompt_detected = true;
                            break;
                        }
                    }

                    if prompt_detected {
                        // 移除最后的提示符
                        for p in &self.config.shell_prompts {
                            if let Some(pos) = output.rfind(p) {
                                output.truncate(pos);
                                break;
                            }
                        }
                        break;
                    }
                }
                Ok(Err(e)) => {
                    return Err(TelnetError::CommandError(format!(
                        "读取命令输出失败: {}", e
                    )));
                }
                Err(_) => {
                    // 超时，继续等待
                    continue;
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // 清理 ANSI 转义序列
        let clean_output = if self.config.clean_ansi {
            clean_ansi_sequences(&output)
        } else {
            output.clone()
        };

        Ok(CommandResult::new(&clean_output, duration_ms))
    }

    /// 下载文件（使用 base64 编码支持二进制文件）
    ///
    /// 通过执行 `base64 <file_path>` 命令读取设备文件内容（base64 编码），
    /// 然后解码并保存到本地。支持二进制文件。
    ///
    /// # 参数
    /// * `remote_path` - 远程文件路径
    /// * `local_path` - 本地保存路径
    /// * `progress_callback` - 进度回调函数（可选）
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 FileDownloadResult
    pub async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> TelnetResult<crate::telnet::types::FileDownloadResult> {
        use crate::telnet::types::{DownloadProgress, FileDownloadResult};
        use std::fs::File;
        use std::io::Write;
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let start_time = Instant::now();

        // 辅助函数：发送进度更新
        let notify_progress = |progress: DownloadProgress| {
            if let Some(callback) = &progress_callback {
                callback(progress);
            }
        };

        // 阶段 1：检查文件是否存在并获取文件大小
        notify_progress(DownloadProgress {
            remote_path: remote_path.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            progress: 0.0,
            stage: "checking".to_string(),
            message: "检查远程文件...".to_string(),
        });

        let check_cmd = format!("ls -la {}", remote_path);
        match self.execute_command(&check_cmd).await {
            Ok(result) => {
                if result.output.contains("No such file") || result.output.contains("not found") {
                    return Ok(FileDownloadResult::failure(
                        remote_path,
                        &format!("远程文件不存在: {}", remote_path),
                    ));
                }
            }
            Err(e) => {
                return Ok(FileDownloadResult::failure(
                    remote_path,
                    &format!("检查文件失败: {}", e),
                ));
            }
        }

        // 获取文件大小
        let size_cmd = format!("wc -c < {}", remote_path);
        let file_size = match self.execute_command(&size_cmd).await {
            Ok(result) => {
                let size_str = result.output.trim();
                size_str.parse::<u64>().unwrap_or(0)
            }
            Err(_) => 0, // 如果无法获取大小，继续下载
        };

        // 阶段 2：下载文件（使用 base64 编码）
        notify_progress(DownloadProgress {
            remote_path: remote_path.to_string(),
            downloaded_bytes: 0,
            total_bytes: file_size,
            progress: 0.0,
            stage: "downloading".to_string(),
            message: "正在下载文件...".to_string(),
        });

        // 尝试使用 base64 命令（支持二进制文件）
        // 直接尝试执行 base64 命令，检查是否可用
        let test_base64_cmd = format!("base64 -h 2>&1 || echo 'BASE64_NOT_FOUND'");
        let use_base64 = match self.execute_command(&test_base64_cmd).await {
            Ok(result) => !result.output.contains("not found") && !result.output.contains("BASE64_NOT_FOUND"),
            Err(_) => false,
        };

        let file_data = if use_base64 {
            // 使用 base64 编码下载（支持二进制文件）
            let download_cmd = format!("base64 {}", remote_path);
            match self.execute_command(&download_cmd).await {
                Ok(result) => {
                    // 检查输出是否包含错误信息
                    if result.output.contains("not found") || result.output.contains("command not found") {
                        // base64 命令不存在，回退到 cat
                        eprintln!("base64 命令不可用，回退到 cat（仅支持文本文件）");
                        let cat_cmd = format!("cat {}", remote_path);
                        match self.execute_command(&cat_cmd).await {
                            Ok(r) => r.output.into_bytes(),
                            Err(e) => {
                                return Ok(FileDownloadResult::failure(
                                    remote_path,
                                    &format!("下载文件失败: {}", e),
                                ));
                            }
                        }
                    } else {
                        // 成功获取 base64 编码的数据
                        let base64_str = result.output.replace('\n', "").replace('\r', "");
                        match STANDARD.decode(&base64_str) {
                            Ok(data) => data,
                            Err(e) => {
                                return Ok(FileDownloadResult::failure(
                                    remote_path,
                                    &format!("Base64 解码失败: {}", e),
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Ok(FileDownloadResult::failure(
                        remote_path,
                        &format!("执行 base64 命令失败: {}", e),
                    ));
                }
            }
        } else {
            // 如果 base64 命令不存在，回退到 cat（仅适用于文本文件）
            eprintln!("base64 命令不可用，回退到 cat（仅支持文本文件）");
            let cat_cmd = format!("cat {}", remote_path);
            match self.execute_command(&cat_cmd).await {
                Ok(result) => {
                    // 清理输出：移除命令回显和提示符
                    let output = result.output;
                    
                    // 移除第一行（命令回显）
                    let mut lines: Vec<&str> = output.lines().collect();
                    if !lines.is_empty() && lines[0].contains(&cat_cmd) {
                        lines.remove(0);
                    }
                    
                    // 移除最后一行（提示符）
                    // 提示符通常包含用户名、主机名和 #/$/>
                    if !lines.is_empty() {
                        let last_line = lines[lines.len() - 1];
                        // 检查是否是提示符
                        if last_line.contains("#") || last_line.contains("$") || 
                           last_line.contains(">") || last_line.contains("[root@") ||
                           last_line.contains(":~]") {
                            lines.pop();
                        }
                    }
                    
                    let cleaned = lines.join("\n");
                    // 添加换行符（如果原始文件有）
                    if output.ends_with('\n') && !cleaned.ends_with('\n') {
                        format!("{}\n", cleaned).into_bytes()
                    } else {
                        cleaned.into_bytes()
                    }
                }
                Err(e) => {
                    return Ok(FileDownloadResult::failure(
                        remote_path,
                        &format!("下载文件失败: {}", e),
                    ));
                }
            }
        };

        let downloaded_size = file_data.len() as u64;

        // 阶段 3：保存文件
        notify_progress(DownloadProgress {
            remote_path: remote_path.to_string(),
            downloaded_bytes: downloaded_size,
            total_bytes: file_size,
            progress: 0.9,
            stage: "saving".to_string(),
            message: "正在保存文件...".to_string(),
        });

        match File::create(local_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&file_data) {
                    return Ok(FileDownloadResult::failure(
                        remote_path,
                        &format!("保存文件失败: {}", e),
                    ));
                }
            }
            Err(e) => {
                return Ok(FileDownloadResult::failure(
                    remote_path,
                    &format!("创建本地文件失败: {}", e),
                ));
            }
        }

        // 阶段 4：完成
        let duration_ms = start_time.elapsed().as_millis() as u64;

        notify_progress(DownloadProgress {
            remote_path: remote_path.to_string(),
            downloaded_bytes: downloaded_size,
            total_bytes: file_size,
            progress: 1.0,
            stage: "completed".to_string(),
            message: format!("下载完成，耗时 {}ms", duration_ms),
        });

        Ok(FileDownloadResult::success(
            remote_path,
            local_path,
            downloaded_size,
            duration_ms,
        ))
    }

    /// 挂载 NFS 虚拟机目录到设备
    ///
    /// 执行挂载命令流程：
    /// 1. `mkdir -p {mount_path}` - 创建挂载目录
    /// 2. `mount -t nfs -o nolock {vm_ip}:{nfs_path} {mount_path}` - 挂载 NFS
    ///
    /// # 参数
    /// * `vm_ip` - 虚拟机 IP 地址
    /// * `nfs_path` - NFS 导出路径（默认为 "/nfs"）
    /// * `mount_path` - 设备上的挂载点路径（默认为 "/mnt/nfs"）
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 MountResult
    pub async fn mount_vm(
        &self,
        vm_ip: &str,
        nfs_path: &str,
        mount_path: &str,
    ) -> TelnetResult<crate::telnet::types::MountResult> {
        use crate::telnet::types::MountResult;

        // 检查连接状态
        if !self.is_logged_in().await {
            return Err(TelnetError::NotConnected(
                "请先连接并登录设备".to_string(),
            ));
        }

        // 步骤 1: 创建挂载目录
        let mkdir_cmd = format!("mkdir -p {}", mount_path);
        let mkdir_result = self.execute_command(&mkdir_cmd).await.map_err(|e| {
            TelnetError::CommandError(format!("创建挂载目录失败: {}", e))
        })?;

        // 步骤 2: 执行挂载
        let mount_cmd = format!(
            "mount -t nfs -o nolock {}:{} {}",
            vm_ip, nfs_path, mount_path
        );
        let mount_result = match self.execute_command(&mount_cmd).await {
            Ok(result) => result,
            Err(e) => {
                return Err(TelnetError::CommandError(format!("NFS 挂载失败: {}", e)));
            }
        };

        // 合并输出
        let combined_output = format!(
            "创建目录:\n{}\n挂载命令: {}\n输出:\n{}",
            mkdir_result.output, mount_cmd, mount_result.output
        );

        // 检查输出中是否包含错误
        if mount_result.output.to_lowercase().contains("error")
            || mount_result.output.to_lowercase().contains("failed")
            || mount_result.output.to_lowercase().contains("mount.nfs:")
                && !mount_result.output.to_lowercase().contains("already mounted")
        {
            return Ok(MountResult::failure(vm_ip, mount_path, &combined_output));
        }

        // 验证挂载是否成功
        let check_cmd = format!("mount | grep {}", mount_path);
        match self.execute_command(&check_cmd).await {
            Ok(check_result) => {
                if check_result.output.contains(mount_path) {
                    Ok(MountResult::success(vm_ip, mount_path, &combined_output))
                } else {
                    // 挂载可能失败，但没检测到明显的错误信息
                    Ok(MountResult::failure(
                        vm_ip,
                        mount_path,
                        &format!("挂载后未检测到挂载点:\n{}", check_result.output),
                    ))
                }
            }
            Err(_) => {
                // 无法验证，但 mount 命令本身没有报错，视为成功
                Ok(MountResult::success(vm_ip, mount_path, &combined_output))
            }
        }
    }

    /// 断开连接
    ///
    /// 关闭 TCP 连接并更新状态。
    pub async fn disconnect(&self) -> TelnetResult<()> {
        let mut stream_guard = self.stream.lock().await;
        // 取出 stream，drop 时会自动关闭连接
        let _ = stream_guard.take();
        drop(stream_guard);

        *self.status.lock().await = ConnectionStatus::Disconnected;

        Ok(())
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        let status = self.status.lock().await;
        matches!(*status, ConnectionStatus::Connected | ConnectionStatus::LoggedIn)
    }

    /// 是否已登录
    pub async fn is_logged_in(&self) -> bool {
        let status = self.status.lock().await;
        matches!(*status, ConnectionStatus::LoggedIn)
    }

    /// 获取连接状态
    pub async fn get_status(&self) -> ConnectionStatus {
        self.status.lock().await.clone()
    }
}

/// 清理 ANSI 转义序列
fn clean_ansi_sequences(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // 跳过 ESC 序列
            if let Some(next) = chars.next() {
                if next == '[' {
                    // CSI 序列：跳过直到遇到字母
                    for c in chars.by_ref() {
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}
