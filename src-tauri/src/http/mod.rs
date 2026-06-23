//! HTTP 服务模块
//!
//! 该模块基于 [axum](https://github.com/tokio-rs/axum) 框架实现，
//! 采用分层架构组织代码，各层职责清晰、易于维护和扩展。
//!
//! # 分层架构总览
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    客户端请求                            │
//! └────────────────────────┬────────────────────────────────┘
//!                          ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Server 层 (server.rs)                                  │
//! │  - TCP 监听与连接管理                                    │
//! │  - 优雅关闭                                              │
//! │  - axum::serve 启动                                      │
//! └────────────────────────┬────────────────────────────────┘
//!                          ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Middleware 层 (middleware.rs)                          │
//! │  - 日志中间件（记录请求/响应）                           │
//! │  - 请求 ID 中间件（生成唯一 ID）                         │
//! │  - 计时中间件（记录耗时）                                │
//! │  - CORS 中间件（tower-http）                            │
//! │  - Trace 中间件（tower-http）                           │
//! └────────────────────────┬────────────────────────────────┘
//!                          ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Router 层 (router.rs)                                  │
//! │  - 路由注册（GET/POST/PUT/DELETE）                       │
//! │  - 路由分组（nest 嵌套）                                 │
//! │  - 路径参数提取（:id）                                   │
//! │  - 中间件作用域管理                                      │
//! └────────────────────────┬────────────────────────────────┘
//!                          ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Handler 层 (handler.rs)                                │
//! │  - 业务逻辑处理                                          │
//! │  - 请求提取器（State/Path/Query/Json）                  │
//! │  - 响应构造（ApiResponse/AppError）                      │
//! │  - 数据模型与存储                                        │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # 模块结构
//!
//! ```text
//! http/
//! ├── mod.rs          # 模块入口，定义 HttpService 门面
//! ├── server.rs       # 服务器层：TCP 监听、优雅关闭
//! ├── middleware.rs   # 中间件层：日志、请求 ID、计时
//! ├── router.rs       # 路由层：路由注册、分组、中间件组装
//! └── handler.rs      # 处理器层：业务逻辑、提取器、响应
//! ```
//!
//! # 快速开始
//!
//! ```ignore
//! use crate::http::HttpService;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 创建并启动 HTTP 服务
//!     let service = HttpService::new("0.0.0.0:8080");
//!     service.start().await?;
//!     Ok(())
//! }
//! ```
//!
//! # 设计要点
//!
//! ## 1. 为什么用 axum？
//!
//! - **官方维护**：tokio 团队出品，与 tokio/hyper 深度集成
//! - **类型安全**：提取器模式在编译期检查参数匹配
//! - **生态完善**：与 tower 中间件生态无缝集成
//! - **性能优秀**：基于 hyper，性能接近裸 HTTP 实现
//!
//! ## 2. 为什么分层？
//!
//! - **单一职责**：每层只做一件事，降低复杂度
//! - **可测试性**：各层可独立测试
//! - **可替换性**：例如可替换 Handler 层实现不同业务
//! - **可扩展性**：新增功能只需在对应层添加
//!
//! ## 3. 状态共享
//!
//! 使用 axum 的 `State` 提取器，通过 `Arc` 共享状态：
//! - `AppState` 持有 `Arc<Mutex<UserStore>>`
//! - 每个请求通过 `State(state): State<AppState>` 提取
//! - `Arc` 廉价克隆，`Mutex` 保证并发安全

pub mod handler;
pub mod middleware;
pub mod router;
pub mod server;

// 重新导出常用类型，方便外部使用
pub use handler::{AppState, ApiResponse, AppError, User, UserStore};
pub use server::{HttpServer, ServerConfig};

// 导入日志宏
use crate::{log_info};

// ============================================================
// HttpService 门面
// ============================================================

/// HTTP 服务门面
///
/// 对外提供统一的 HTTP 服务接口，封装了服务器创建和启动逻辑。
/// 内部组合了 `HttpServer` 和 `AppState`。
///
/// # 设计模式
/// 采用"门面模式"（Facade），为复杂的子系统提供简化接口：
/// - 调用方无需关心 Router、Middleware、Handler 的组装细节
/// - 只需调用 `new()` + `start()` 即可启动服务
///
/// # 示例
/// ```ignore
/// use crate::http::HttpService;
///
/// let service = HttpService::new("0.0.0.0:8080");
/// service.start().await?;
/// ```
pub struct HttpService {
    /// 内部的 HTTP 服务器
    server: HttpServer,
}

impl HttpService {
    /// 创建新的 HTTP 服务
    ///
    /// 使用默认的应用状态（空的用户存储）。
    ///
    /// # 参数
    /// * `addr` - 监听地址（如 `0.0.0.0:8080`）
    ///
    /// # 示例
    /// ```ignore
    /// let service = HttpService::new("127.0.0.1:3000");
    /// ```
    pub fn new(addr: &str) -> Self {
        let state = AppState::new();
        let server = HttpServer::new(addr, state);
        Self { server }
    }

    /// 使用自定义应用状态创建 HTTP 服务
    ///
    /// 当需要共享外部数据（如数据库连接池）时使用。
    ///
    /// # 参数
    /// * `addr` - 监听地址
    /// * `state` - 自定义应用状态
    pub fn with_state(addr: &str, state: AppState) -> Self {
        let server = HttpServer::new(addr, state);
        Self { server }
    }

    /// 启动 HTTP 服务
    ///
    /// 该方法会阻塞当前异步上下文，持续处理请求。
    /// 启用优雅关闭时，收到 Ctrl+C 信号会优雅退出。
    ///
    /// # 返回值
    /// 返回 `anyhow::Result<()>`，仅在启动或运行错误时返回
    ///
    /// # 错误处理
    /// - 端口被占用：返回绑定错误
    /// - 服务器内部错误：返回运行错误
    pub async fn start(&self) -> anyhow::Result<()> {
        log_info!("正在启动 HTTP 服务...");
        self.server.serve().await
    }

    /// 获取监听地址
    pub fn bind_addr(&self) -> String {
        self.server.bind_addr()
    }
}

// ============================================================
// 模块测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service() {
        let service = HttpService::new("0.0.0.0:8080");
        assert_eq!(service.bind_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        // 验证状态可以克隆（Router 需要克隆状态）
        let _state2 = state.clone();
    }
}
