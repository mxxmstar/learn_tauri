//! Telnet 配置管理
//!
//! 定义 telnet 连接的配置选项，包括超时时间、提示符、缓冲区大小等。

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;

/// Telnet 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelnetConfig {
    /// 目标设备地址（IP:端口）
    pub addr: String,
    /// 连接超时（毫秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    /// 登录超时（毫秒）
    #[serde(default = "default_login_timeout")]
    pub login_timeout_ms: u64,
    /// 命令执行超时（毫秒）
    #[serde(default = "default_command_timeout")]
    pub command_timeout_ms: u64,
    /// 读取缓冲区大小（字节）
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// 登录用户名提示符（用于检测是否需要输入用户名）
    #[serde(default = "default_login_prompt")]
    pub login_prompt: String,
    /// 登录密码提示符（用于检测是否需要输入密码）
    #[serde(default = "default_password_prompt")]
    pub password_prompt: String,
    /// Shell 提示符列表（用于检测命令执行完成）
    #[serde(default = "default_shell_prompts")]
    pub shell_prompts: Vec<String>,
    /// 是否清理 ANSI 转义序列
    #[serde(default = "default_clean_ansi")]
    pub clean_ansi: bool,
}

impl Default for TelnetConfig {
    fn default() -> Self {
        Self {
            addr: "192.168.1.1:23".to_string(),
            connect_timeout_ms: default_connect_timeout(),
            login_timeout_ms: default_login_timeout(),
            command_timeout_ms: default_command_timeout(),
            buffer_size: default_buffer_size(),
            login_prompt: default_login_prompt(),
            password_prompt: default_password_prompt(),
            shell_prompts: default_shell_prompts(),
            clean_ansi: default_clean_ansi(),
        }
    }
}

impl TelnetConfig {
    /// 创建新的配置
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
            ..Default::default()
        }
    }

    /// 设置连接超时
    pub fn with_connect_timeout(mut self, timeout_ms: u64) -> Self {
        self.connect_timeout_ms = timeout_ms;
        self
    }

    /// 设置登录超时
    pub fn with_login_timeout(mut self, timeout_ms: u64) -> Self {
        self.login_timeout_ms = timeout_ms;
        self
    }

    /// 设置命令执行超时
    pub fn with_command_timeout(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }

    /// 添加自定义 shell 提示符
    pub fn with_shell_prompt(mut self, prompt: &str) -> Self {
        self.shell_prompts.push(prompt.to_string());
        self
    }

    /// 解析地址为 SocketAddr
    pub fn parse_addr(&self) -> Result<SocketAddr, String> {
        SocketAddr::from_str(&self.addr)
            .map_err(|e| format!("地址解析失败: {} ({})", e, self.addr))
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        // 验证地址格式
        self.parse_addr()?;

        // 验证超时时间
        if self.connect_timeout_ms == 0 {
            return Err("连接超时时间不能为 0".to_string());
        }
        if self.login_timeout_ms == 0 {
            return Err("登录超时时间不能为 0".to_string());
        }
        if self.command_timeout_ms == 0 {
            return Err("命令执行超时时间不能为 0".to_string());
        }

        // 验证缓冲区大小
        if self.buffer_size < 1024 {
            return Err("缓冲区大小不能小于 1024 字节".to_string());
        }

        Ok(())
    }
}

// 默认值函数
fn default_connect_timeout() -> u64 {
    10000 // 10 秒
}

fn default_login_timeout() -> u64 {
    15000 // 15 秒
}

fn default_command_timeout() -> u64 {
    30000 // 30 秒
}

fn default_buffer_size() -> usize {
    8192 // 8 KB
}

fn default_login_prompt() -> String {
    "login:".to_string()
}

fn default_password_prompt() -> String {
    "Password:".to_string()
}

fn default_shell_prompts() -> Vec<String> {
    vec![
        "# ".to_string(),      // root 用户
        "$ ".to_string(),      // 普通用户
        "> ".to_string(),      // 某些设备
        "~# ".to_string(),     // root 用户（home 目录）
        "~$ ".to_string(),     // 普通用户（home 目录）
    ]
}

fn default_clean_ansi() -> bool {
    true
}
