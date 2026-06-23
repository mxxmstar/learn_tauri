//! HTTP 中间件模块
//!
//! 该模块基于 axum 的 `middleware::from_fn` 实现，包括：
//! - 自定义中间件（日志、请求 ID、计时）
//! - 中间件使用说明
//!
//! # 中间件层在整个架构中的位置
//!
//! ```text
//! 请求 → Server → [Middleware链] → Router → Handler
//!                    ↑
//!               （当前模块）
//! ```
//!
//! # axum 中间件的实现方式
//!
//! axum 使用普通 async 函数作为中间件，通过 `axum::middleware::from_fn` 包装。
//! 函数签名固定为：
//!
//! ```ignore
//! use axum::{extract::Request, middleware::Next, response::Response};
//!
//! async fn my_middleware(req: Request, next: Next) -> Response {
//!     // 前置逻辑（可读取/修改请求）
//!     let response = next.run(req).await;
//!     // 后置逻辑（可修改响应）
//!     response
//! }
//! ```
//!
//! # 中间件执行顺序（洋葱模型）
//!
//! ```text
//! 请求 → 中间件A（前置）
//!   请求 → 中间件B（前置）
//!     请求 → Handler → 响应
//!   响应 ← 中间件B（后置）
//! 响应 ← 中间件A（后置）
//! ```
//!
//! # 使用方式
//!
//! 中间件通过 `Router::layer()` 方法添加。
//! **注意**：axum 中 `layer()` 的添加顺序与执行顺序相反——
//! 后添加的 layer 先执行（最内层），先添加的 layer 后执行（最外层）。
//!
//! ```ignore
//! use axum::Router;
//! use axum::middleware::from_fn;
//! use crate::http::middleware::{logging_middleware, request_id_middleware};
//!
//! let app = Router::new()
//!     .route("/api/users", get(get_users))
//!     .layer(from_fn(logging_middleware))        // 先添加 = 后执行（最外层）
//!     .layer(from_fn(request_id_middleware));     // 后添加 = 先执行（最内层）
//! ```

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

// 导入日志宏
use crate::{log_info, log_error, log_debug};

// ============================================================
// 日志中间件
// ============================================================

/// 日志中间件函数
///
/// 记录每个请求的方法、路径、状态码和耗时。
///
/// # 工作流程
/// 1. 记录请求开始信息（方法、路径）
/// 2. 记录开始时间
/// 3. 调用 `next.run(req)` 将请求传递给后续处理
/// 4. 记录响应信息（状态码、耗时）
///
/// # 返回值
/// 返回 `Response`，透传后续处理器的响应
///
/// # 使用方式
/// ```ignore
/// use axum::middleware::from_fn;
/// router.layer(from_fn(logging_middleware));
/// ```
pub async fn logging_middleware(req: Request, next: Next) -> Response {
    // ---- 前置逻辑：记录请求信息 ----
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    log_info!("→ {} {}", method, uri.path());

    // 调用后续处理（下一个中间件或路由处理器）
    let response = next.run(req).await;

    // ---- 后置逻辑：记录响应信息 ----
    let duration = start.elapsed();
    let status = response.status();

    log_info!(
        "← {} {} {} ({:?})",
        method,
        uri.path(),
        status,
        duration
    );

    response
}

// ============================================================
// 请求 ID 中间件
// ============================================================

/// 请求 ID 中间件函数
///
/// 为每个请求生成或复用唯一的请求 ID，便于日志追踪。
///
/// # 工作流程
/// 1. 检查请求头 `X-Request-Id`，存在则复用，否则生成新 UUID
/// 2. 将请求 ID 写入响应头 `X-Request-Id`
///
/// # 好处
/// - 分布式系统中可通过请求 ID 串联一次请求的所有日志
/// - 客户端可从响应头获取请求 ID，便于问题排查
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    // 检查请求头中是否已有 Request-Id
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // 生成新的 UUID 作为请求 ID
            Uuid::new_v4().to_string()
        });

    log_debug!("请求 ID: {}", request_id);

    // 将请求 ID 写入请求头（供后续处理器使用）
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert(
            HeaderName::from_static("x-request-id"),
            value,
        );
    }

    // 调用后续处理
    let mut response = next.run(req).await;

    // 在响应头中添加请求 ID
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(
            HeaderName::from_static("x-request-id"),
            value,
        );
    }

    response
}

// ============================================================
// 计时中间件
// ============================================================

/// 请求计时中间件函数
///
/// 记录请求处理耗时，并在响应头 `X-Response-Time` 中返回（毫秒）。
///
/// 与 `logging_middleware` 的区别：本中间件将耗时写入响应头，
/// 客户端可直接获取；日志中间件仅记录到日志。
pub async fn timing_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();

    // 在 move 前保存请求信息（next.run(req) 会消费 req）
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let mut response = next.run(req).await;

    let duration_ms = start.elapsed().as_millis();

    // 在响应头中添加耗时信息
    if let Ok(value) = HeaderValue::from_str(&duration_ms.to_string()) {
        response.headers_mut().insert(
            HeaderName::from_static("x-response-time"),
            value,
        );
    }

    // 超过 1 秒的慢请求记录警告
    if duration_ms > 1000 {
        log_error!(
            "慢请求: {} {} 耗时 {}ms",
            method,
            path,
            duration_ms
        );
    } else {
        log_debug!("请求耗时: {}ms", duration_ms);
    }

    response
}
