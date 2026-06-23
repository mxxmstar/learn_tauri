//! HTTP 服务器模块
//!
//! 该模块基于 axum 的 `serve` 函数实现服务器层，包括：
//! - TCP 监听与启动
//! - 优雅关闭（Graceful Shutdown）
//! - 服务器配置
//! - `HttpServer` 封装结构体
//!
//! # 服务器层在整个架构中的位置
//!
//! ```text
//! 请求 → [Server] → Middleware链 → Router → Handler
//!           ↑
//!       （当前模块）
//! ```
//!
//! # 请求处理流程
//!
//! axum 内部已经完整实现了 HTTP 协议解析和请求分发，本模块负责：
//! 1. 绑定 TCP 监听地址
//! 2. 配置优雅关闭信号
//! 3. 启动 axum 服务
//!
//! ```text
//! TCP连接接入 → axum解析HTTP请求 → 执行中间件链 → 路由匹配 → 处理器 → 响应
//! ```
//!
//! # 优雅关闭
//!
//! 服务器在收到 Ctrl+C 或 SIGTERM 信号时，会：
//! 1. 停止接受新连接
//! 2. 等待正在处理的请求完成
//! 3. 关闭服务器
//!
//! # 示例
//!
//! ```ignore
//! use crate::http::server::HttpServer;
//! use crate::http::handler::AppState;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let state = AppState::new();
//!     let server = HttpServer::new("0.0.0.0:8080", state);
//!     server.serve().await?;
//!     Ok(())
//! }
//! ```

use anyhow::{Result, Context as _};
use tokio::net::TcpListener;

use crate::http::handler::AppState;
use crate::http::router::build_app_router;

// 导入日志宏
use crate::{log_info};

// ============================================================
// 服务器配置
// ============================================================

/// HTTP 服务器配置
///
/// 集中管理服务器的各项配置参数。
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 监听地址（如 `0.0.0.0` 表示所有网卡，`127.0.0.1` 表示仅本机）
    pub host: String,
    /// 监听端口（如 `8080`）
    pub port: u16,
    /// 是否启用优雅关闭
    pub graceful_shutdown: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            graceful_shutdown: true,
        }
    }
}

impl ServerConfig {
    /// 构建监听地址字符串
    ///
    /// 返回 `host:port` 格式的字符串，如 `0.0.0.0:8080`
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// ============================================================
// HTTP 服务器
// ============================================================

/// HTTP 服务器
///
/// 封装了 axum 的服务器启动逻辑，提供简洁的启动接口。
///
/// # 生命周期
/// 1. `new()`：创建服务器实例
/// 2. `with_config()`：自定义配置（可选）
/// 3. `serve()`：启动服务器，阻塞当前异步上下文
pub struct HttpServer {
    /// 服务器配置
    config: ServerConfig,
    /// 应用共享状态
    state: AppState,
}

impl HttpServer {
    /// 创建新的 HTTP 服务器
    ///
    /// 使用默认配置（监听 `0.0.0.0:8080`，启用优雅关闭）。
    ///
    /// # 参数
    /// * `addr` - 监听地址（如 `0.0.0.0:8080`）
    /// * `state` - 应用共享状态
    pub fn new(addr: &str, state: AppState) -> Self {
        let (host, port) = parse_addr(addr);
        Self {
            config: ServerConfig {
                host,
                port,
                graceful_shutdown: true,
            },
            state,
        }
    }

    /// 自定义服务器配置（Builder 模式）
    ///
    /// # 参数
    /// * `config` - 服务器配置
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// 获取监听地址
    pub fn bind_addr(&self) -> String {
        self.config.bind_addr()
    }

    /// 启动 HTTP 服务器
    ///
    /// 该方法会阻塞当前异步上下文，持续监听并处理请求。
    /// 根据配置决定是否启用优雅关闭。
    ///
    /// # 返回值
    /// 返回 `Result<()>`：
    /// - 绑定地址失败：返回错误
    /// - 服务器正常运行期间不会返回
    ///
    /// # 优雅关闭
    /// 启用后，收到 Ctrl+C 信号会触发优雅关闭：
    /// 1. 停止接受新连接
    /// 2. 等待活跃请求完成
    /// 3. 退出服务器
    pub async fn serve(&self) -> Result<()> {
        // 1. 绑定 TCP 监听器
        let addr_str = self.config.bind_addr();
        let listener = TcpListener::bind(&addr_str)
            .await
            .context(format!("无法绑定到地址 {}", addr_str))?;

        let addr = listener.local_addr()
            .context("获取本地地址失败")?;

        // 2. 构建应用路由器（包含所有路由和中间件）
        let app = build_app_router(self.state.clone());

        // 3. 打印启动信息
        log_info!("════════════════════════════════════════");
        log_info!("HTTP 服务器已启动");
        log_info!("监听地址: http://{}", addr);
        log_info!("优雅关闭: {}", if self.config.graceful_shutdown { "已启用" } else { "未启用" });
        log_info!("────────────────────────────────────────");
        log_info!("已注册路由:");
        for (method, path) in crate::http::router::list_routes() {
            log_info!("  {:6} {}", method, path);
        }
        log_info!("────────────────────────────────────────");
        log_info!("提示: 按 Ctrl+C 优雅关闭服务器");
        log_info!("════════════════════════════════════════");

        // 4. 启动 axum 服务
        if self.config.graceful_shutdown {
            // 启用优雅关闭：监听 Ctrl+C 信号
            let shutdown = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("无法安装 Ctrl+C 信号处理器");
                log_info!("收到关闭信号，正在优雅关闭服务器...");
            };

            // axum 的 serve 返回 Serve 结构体，调用 await 开始运行
            let serve = axum::serve(listener, app);
            serve
                .with_graceful_shutdown(shutdown)
                .await
                .context("服务器运行错误")?;
        } else {
            // 不启用优雅关闭
            axum::serve(listener, app)
                .await
                .context("服务器运行错误")?;
        }

        log_info!("HTTP 服务器已停止");
        Ok(())
    }
}

/// 解析地址字符串
///
/// 将 `host:port` 格式的字符串分割为主机和端口。
///
/// # 参数
/// * `addr` - 地址字符串（如 `0.0.0.0:8080`）
///
/// # 返回值
/// 返回 `(host, port)` 元组
fn parse_addr(addr: &str) -> (String, u16) {
    if let Some(colon) = addr.rfind(':') {
        let host = addr[..colon].to_string();
        let port = addr[colon + 1..].parse().unwrap_or(8080);
        (host, port)
    } else {
        ("0.0.0.0".to_string(), 8080)
    }
}
