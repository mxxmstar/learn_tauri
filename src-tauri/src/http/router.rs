//! HTTP 路由模块
//!
//! 该模块基于 axum 的 `Router` 实现路由层，包括：
//! - 路由注册（GET、POST、PUT、DELETE 等）
//! - 路由分组（`nest` 嵌套路由）
//! - 全局中间件应用
//! - 应用路由构建入口 `build_app_router`
//!
//! # 路由层在整个架构中的位置
//!
//! ```text
//! 请求 → Server → Middleware链 → [Router] → Handler
//!                                  ↑
//!                             （当前模块）
//! ```
//!
//! # axum 路由的特点
//!
//! axum 的 `Router` 提供了强大的路由功能：
//! - **路径参数**：`/users/:id` 自动提取 `id`
//! - **路由嵌套**：`nest("/api", sub_router)` 实现模块化路由
//! - **中间件作用域**：`layer()` 应用的中间件仅对该路由器内的路由生效
//! - **类型安全的提取器**：编译期检查提取器与处理器签名匹配
//!
//! # 路由模式
//!
//! | 模式 | 示例 | 说明 |
//! |------|------|------|
//! | 静态路由 | `/api/users` | 精确匹配 |
//! | 参数路由 | `/api/users/:id` | `:id` 捕获路径段 |
//! | 通配符路由 | `/files/*path` | `*path` 捕获剩余路径 |
//!
//! # 示例
//!
//! ```ignore
//! use crate::http::router::build_app_router;
//! use crate::http::handler::AppState;
//!
//! let state = AppState::new();
//! let app = build_app_router(state);
//! // 将 app 传给 server 启动
//! ```

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::http::handler::AppState;
use crate::http::middleware::{logging_middleware, request_id_middleware, timing_middleware};

// 导入日志宏
use crate::log_info;

// ============================================================
// 业务路由构建
// ============================================================

/// 构建用户管理路由
///
/// 定义所有 `/api/users` 相关的路由。
/// 通过 `nest` 嵌套到主路由器中，实现路由分组。
///
/// # 路由列表
/// | 方法 | 路径 | 处理器 | 说明 |
/// |------|------|--------|------|
/// | GET | /api/users | list_users | 获取用户列表 |
/// | GET | /api/users/:id | get_user | 获取单个用户 |
/// | POST | /api/users | create_user | 创建用户 |
/// | PUT | /api/users/:id | update_user | 更新用户 |
/// | DELETE | /api/users/:id | delete_user | 删除用户 |
///
/// # 参数
/// 无（处理器通过 `State` 提取器获取共享状态）
///
/// # 返回值
/// 返回配置好用户路由的 `Router`
fn build_user_routes() -> Router<AppState> {
    Router::new()
        // GET /api/users - 获取用户列表（支持 ?name=xxx 过滤）
        .route("/", get(crate::http::handler::list_users))
        // GET /api/users/:id - 获取单个用户
        // POST /api/users - 创建用户
        // PUT /api/users/:id - 更新用户
        // DELETE /api/users/:id - 删除用户
        // 注意：同一个路径可以绑定多个方法，通过链式调用实现
        .route(
            "/:id",
            get(crate::http::handler::get_user)
                .put(crate::http::handler::update_user)
                .delete(crate::http::handler::delete_user),
        )
        // POST /api/users（创建用户路径与列表不同，单独定义）
        .route("/create", post(crate::http::handler::create_user))
}

// ============================================================
// 应用主路由构建
// ============================================================

/// 构建应用主路由器
///
/// 这是路由层的总入口，组装所有路由和中间件。
/// 服务器层（`server.rs`）调用此函数获取最终的 `Router` 实例。
///
/// # 路由结构
/// ```text
/// /                  → root()           # 欢迎页
/// /health            → health_check()   # 健康检查
/// /api/users/*       → 用户管理路由      # 业务路由
/// ```
///
/// # 中间件应用顺序（从外到内执行）
/// 1. TraceLayer（tower-http，配合 tracing 输出请求日志）
/// 2. CorsLayer（CORS 跨域处理）
/// 3. 日志中间件（自定义）
/// 4. 请求 ID 中间件（自定义）
/// 5. 计时中间件（自定义）
///
/// **注意**：axum 中 `layer()` 的添加顺序与执行顺序相反——
/// 先添加的 layer 后执行（最外层），后添加的 layer 先执行（最内层）。
///
/// # 参数
/// * `state` - 应用共享状态，通过 `with_state` 注入到所有处理器
///
/// # 返回值
/// 返回 `Router<()>`，可直接传给 axum 的 `serve` 函数
pub fn build_app_router(state: AppState) -> Router<()> {
    log_info!("正在构建应用路由器...");

    // 1. 构建业务路由（用户管理）
    let user_routes = build_user_routes();

    // 2. 组装主路由器
    let app = Router::new()
        // 根路径 - 服务信息
        .route("/", get(crate::http::handler::root))
        // 健康检查端点
        .route("/health", get(crate::http::handler::health_check))
        // 嵌套用户管理路由到 /api/users 前缀下
        .nest("/api/users", user_routes)
        // ---- 应用全局中间件 ----
        // 注意顺序：先添加 = 后执行（外层），后添加 = 先执行（内层）
        // TraceLayer：tower-http 提供的请求追踪，配合 tracing 使用
        .layer(TraceLayer::new_for_http())
        // CorsLayer：处理 CORS 跨域请求
        .layer(CorsLayer::permissive())
        // 自定义日志中间件（最外层，记录完整请求-响应周期）
        .layer(from_fn(logging_middleware))
        // 请求 ID 中间件
        .layer(from_fn(request_id_middleware))
        // 计时中间件（最内层，紧贴业务处理）
        .layer(from_fn(timing_middleware))
        // 注入应用共享状态
        .with_state(state);

    log_info!("应用路由器构建完成");
    app
}

/// 列出所有已注册的路由（用于启动日志和调试）
///
/// axum 0.7 暂未提供官方的路由列表 API，此函数返回静态路由文档。
/// 生产环境可考虑使用 `axum-route-stats` 等第三方库。
pub fn list_routes() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GET", "/"),
        ("GET", "/health"),
        ("GET", "/api/users"),
        ("GET", "/api/users/:id"),
        ("POST", "/api/users/create"),
        ("PUT", "/api/users/:id"),
        ("DELETE", "/api/users/:id"),
    ]
}
