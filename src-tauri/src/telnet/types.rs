//! Telnet 数据类型定义
//!
//! 定义 telnet 模块使用的各种数据类型，包括登录结果、命令结果、文件下载结果等。

use serde::{Deserialize, Serialize};

/// 登录结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    /// 是否登录成功
    pub success: bool,
    /// 登录后的提示符（用于后续命令执行）
    pub prompt: String,
    /// 登录过程中的输出信息
    pub output: String,
}

impl LoginResult {
    pub fn success(prompt: &str, output: &str) -> Self {
        Self {
            success: true,
            prompt: prompt.to_string(),
            output: output.to_string(),
        }
    }

    pub fn failure(output: &str) -> Self {
        Self {
            success: false,
            prompt: String::new(),
            output: output.to_string(),
        }
    }
}

/// 命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// 命令退出状态（如果设备支持）
    pub exit_status: Option<i32>,
    /// 命令输出
    pub output: String,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
}

impl CommandResult {
    pub fn new(output: &str, duration_ms: u64) -> Self {
        Self {
            exit_status: None,
            output: output.to_string(),
            duration_ms,
        }
    }

    pub fn with_exit_status(mut self, status: i32) -> Self {
        self.exit_status = Some(status);
        self
    }
}

/// 文件下载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownloadResult {
    /// 是否下载成功
    pub success: bool,
    /// 远程文件路径
    pub remote_path: String,
    /// 本地保存路径
    pub local_path: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 下载耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

/// 下载进度通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// 远程文件路径
    pub remote_path: String,
    /// 已下载字节数
    pub downloaded_bytes: u64,
    /// 文件总大小（字节，如果未知则为 0）
    pub total_bytes: u64,
    /// 下载进度（0.0 - 1.0）
    pub progress: f32,
    /// 当前阶段："checking", "downloading", "saving", "completed", "error"
    pub stage: String,
    /// 状态消息
    pub message: String,
}

impl FileDownloadResult {
    pub fn success(remote_path: &str, local_path: &str, file_size: u64, duration_ms: u64) -> Self {
        Self {
            success: true,
            remote_path: remote_path.to_string(),
            local_path: local_path.to_string(),
            file_size,
            duration_ms,
            error: None,
        }
    }

    pub fn failure(remote_path: &str, error: &str) -> Self {
        Self {
            success: false,
            remote_path: remote_path.to_string(),
            local_path: String::new(),
            file_size: 0,
            duration_ms: 0,
            error: Some(error.to_string()),
        }
    }
}

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接（未登录）
    Connected,
    /// 已登录
    LoggedIn,
    /// 连接错误
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

/// NFS 挂载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountResult {
    /// 是否成功
    pub success: bool,
    /// 虚拟机 IP
    pub vm_ip: String,
    /// 本地挂载路径
    pub mount_path: String,
    /// 执行过程中的输出
    pub output: String,
    /// 错误信息
    pub error: Option<String>,
}

impl MountResult {
    pub fn success(vm_ip: &str, mount_path: &str, output: &str) -> Self {
        Self {
            success: true,
            vm_ip: vm_ip.to_string(),
            mount_path: mount_path.to_string(),
            output: output.to_string(),
            error: None,
        }
    }

    pub fn failure(vm_ip: &str, mount_path: &str, error: &str) -> Self {
        Self {
            success: false,
            vm_ip: vm_ip.to_string(),
            mount_path: mount_path.to_string(),
            output: String::new(),
            error: Some(error.to_string()),
        }
    }
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 设备 IP 地址
    pub ip: String,
    /// 设备主机名
    pub hostname: String,
    /// 设备型号
    pub model: String,
    /// 系统信息
    pub system: String,
    /// 登录用户名
    pub username: String,
}

impl DeviceInfo {
    pub fn new(ip: &str, username: &str) -> Self {
        Self {
            ip: ip.to_string(),
            hostname: String::new(),
            model: String::new(),
            system: String::new(),
            username: username.to_string(),
        }
    }
}

/// Tauri 命令返回结果包装
#[derive(Debug, Clone, Serialize)]
pub struct TelnetOpResult<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> TelnetOpResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}
