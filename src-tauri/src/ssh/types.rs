//! SSH / SFTP 远程文件管理模块的数据类型定义。
//!
//! 这个文件专门保存前后端共享的数据结构，目标是：
//! 1. 让 `mod.rs` 更聚焦于业务流程；
//! 2. 让前后端字段语义保持一致；
//! 3. 让后续继续扩展上传、删除、重命名等能力时更容易维护。

use serde::{Deserialize, Serialize};

/// SSH 连接请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConnectRequest {
    /// 远程主机地址，例如 `192.168.66.117`
    pub host: String,
    /// SSH 端口，默认通常为 `22`
    pub port: u16,
    /// 登录用户名
    pub username: String,
    /// 登录密码
    pub password: String,
    /// 连接前经过用户确认的主机指纹
    pub expected_host_fingerprint: Option<String>,
    /// 连接成功后默认进入的远程目录
    pub initial_path: Option<String>,
}

/// SSH 模块统一命令返回结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshCmdResult<T>
where
    T: Serialize,
{
    /// 是否执行成功
    pub success: bool,
    /// 成功时返回的数据
    pub data: Option<T>,
    /// 失败时的错误信息
    pub error: Option<String>,
}

impl<T> SshCmdResult<T>
where
    T: Serialize,
{
    /// 构造成功结果。
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 构造失败结果。
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// SSH 会话基础信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshSessionInfo {
    /// 当前连接对应的会话 ID
    pub session_id: String,
    /// 远程主机地址
    pub host: String,
    /// 远程端口
    pub port: u16,
    /// 登录用户名
    pub username: String,
    /// 当前远程路径
    pub current_path: String,
    /// 本次连接实际校验通过的主机指纹
    pub host_fingerprint: String,
}

/// SSH 主机指纹探测请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostProbeRequest {
    /// 远程主机地址
    pub host: String,
    /// SSH 端口
    pub port: u16,
}

/// SSH 主机指纹探测结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostProbeResult {
    /// 远程主机地址
    pub host: String,
    /// SSH 端口
    pub port: u16,
    /// 探测到的主机指纹，采用 OpenSSH 常见格式
    pub fingerprint: String,
}

/// 目录列表中的单个文件项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileEntry {
    /// 文件或目录名称
    pub name: String,
    /// 远程完整路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 是否为普通文件
    pub is_file: bool,
    /// 是否为符号链接
    pub is_symlink: bool,
    /// 文件类型描述，例如 `directory` / `file` / `symlink`
    pub file_type: String,
    /// 大小，单位字节
    pub size: u64,
    /// 原始权限位
    pub permissions: Option<u32>,
    /// 八进制权限文本，例如 `755`
    pub permission_text: Option<String>,
    /// 所属用户 ID
    pub uid: Option<u32>,
    /// 所属用户组 ID
    pub gid: Option<u32>,
    /// 修改时间
    pub modified_at: Option<String>,
    /// 访问时间
    pub accessed_at: Option<String>,
}

/// 文件属性详情。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileProperties {
    /// 文件或目录名称
    pub name: String,
    /// 远程完整路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 是否为普通文件
    pub is_file: bool,
    /// 是否为符号链接
    pub is_symlink: bool,
    /// 文件类型说明
    pub file_type: String,
    /// 大小，单位字节
    pub size: u64,
    /// 原始权限位
    pub permissions: Option<u32>,
    /// 八进制权限文本
    pub permission_text: Option<String>,
    /// 所属用户 ID
    pub uid: Option<u32>,
    /// 所属用户组 ID
    pub gid: Option<u32>,
    /// 修改时间
    pub modified_at: Option<String>,
    /// 访问时间
    pub accessed_at: Option<String>,
}

/// 文件下载进度事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// 对应的 SSH 会话 ID
    pub session_id: String,
    /// 远程文件路径
    pub remote_path: String,
    /// 本地保存路径
    pub local_path: String,
    /// 已下载字节数
    pub downloaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 进度比例，范围 `0.0 ~ 1.0`
    pub progress: f64,
    /// 阶段说明：checking / downloading / saving / completed / error
    pub stage: String,
    /// 面向用户的提示文本
    pub message: String,
}

/// 文件下载完成结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDownloadResult {
    /// 远程文件路径
    pub remote_path: String,
    /// 本地保存路径
    pub local_path: String,
    /// 文件大小
    pub file_size: u64,
    /// 整个下载耗时
    pub duration_ms: u64,
}

/// 文件上传进度事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    /// 对应的 SSH 会话 ID
    pub session_id: String,
    /// 本地源文件路径
    pub local_path: String,
    /// 远程目标文件路径
    pub remote_path: String,
    /// 已上传字节数
    pub uploaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 进度比例，范围 `0.0 ~ 1.0`
    pub progress: f64,
    /// 阶段说明：checking / uploading / saving / completed / error
    pub stage: String,
    /// 面向用户的提示文本
    pub message: String,
}

/// 文件上传完成结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadResult {
    /// 本地源文件路径
    pub local_path: String,
    /// 远程目标文件路径
    pub remote_path: String,
    /// 文件大小
    pub file_size: u64,
    /// 整个上传耗时
    pub duration_ms: u64,
}

/// 第三版新增：创建远程目录成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectoryResult {
    /// 新建成功后的远程目录完整路径
    pub path: String,
}

/// 第三版新增：重命名远程路径成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePathResult {
    /// 重命名前的远程路径
    pub old_path: String,
    /// 重命名后的远程路径
    pub new_path: String,
}

/// 第三版新增：删除远程路径成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePathResult {
    /// 已删除的远程路径
    pub path: String,
    /// 删除目标是否为目录
    pub is_dir: bool,
}

/// 建议的本地下载路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedDownloadPath {
    /// 建议保存到本机的完整路径
    pub suggested_path: String,
}

/// 文本文件本地打开结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFileResult {
    /// 远程文件路径
    pub remote_path: String,
    /// 本地缓存路径
    pub local_path: String,
    /// 文件大小
    pub file_size: u64,
    /// 是否识别为文本文件
    pub is_text: bool,
    /// 文本内容
    pub text_content: Option<String>,
}
