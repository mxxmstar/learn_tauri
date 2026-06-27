//! SSH / SFTP 远程文件管理模块的数据类型定义。
//!
//! 这个文件专门负责保存前后端共享的数据结构，
//! 这样业务实现文件可以更聚焦于流程逻辑本身。

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
    /// 连接前经用户确认的主机指纹
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
    /// 会话 ID
    pub session_id: String,
    /// 远程主机地址
    pub host: String,
    /// SSH 端口
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
    /// 探测到的主机指纹
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
    /// 文件类型说明
    pub file_type: String,
    /// 文件大小，单位字节
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
    /// 文件大小，单位字节
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
    /// 进度比例
    pub progress: f64,
    /// 当前阶段
    pub stage: String,
    /// 给用户看的提示信息
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
    /// 本次下载耗时
    pub duration_ms: u64,
}

/// 第五版新增：远程目录递归下载完成结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryDownloadResult {
    /// 被递归下载的远程根目录路径
    pub remote_path: String,
    /// 本地最终落地的目录路径
    pub local_path: String,
    /// 本次递归下载包含的文件数量
    pub file_count: u64,
    /// 本次递归下载包含的目录数量（包含根目录）
    pub directory_count: u64,
    /// 全部文件累计字节数
    pub total_bytes: u64,
    /// 本次递归下载耗时
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
    /// 进度比例
    pub progress: f64,
    /// 当前阶段
    pub stage: String,
    /// 给用户看的提示信息
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
    /// 本次上传耗时
    pub duration_ms: u64,
}

/// 第五版新增：本地目录递归上传完成结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryUploadResult {
    /// 本地递归上传的根目录路径
    pub local_path: String,
    /// 远程最终创建 / 覆盖的根目录路径
    pub remote_path: String,
    /// 本次递归上传包含的文件数量
    pub file_count: u64,
    /// 本次递归上传包含的目录数量（包含根目录）
    pub directory_count: u64,
    /// 全部文件累计字节数
    pub total_bytes: u64,
    /// 本次递归上传耗时
    pub duration_ms: u64,
}

/// 创建远程目录成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectoryResult {
    /// 新建成功后的远程目录完整路径
    pub path: String,
}

/// 重命名成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePathResult {
    /// 原始远程路径
    pub old_path: String,
    /// 重命名后的远程路径
    pub new_path: String,
}

/// 删除成功结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePathResult {
    /// 被删除的远程路径
    pub path: String,
    /// 删除目标是否为目录
    pub is_dir: bool,
}

/// 建议的本地下载路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedDownloadPath {
    /// 推荐保存到本机的完整路径
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

/// 第四版新增：远程文本文件保存结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRemoteTextResult {
    /// 被保存的远程文件路径
    pub remote_path: String,
    /// 保存后的文件大小
    pub file_size: u64,
    /// 本次保存耗时
    pub duration_ms: u64,
}
