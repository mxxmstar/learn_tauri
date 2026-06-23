//! HTTP 处理器模块
//!
//! 该模块基于 axum 框架实现处理器层，包括：
//! - 通用响应包装结构 `ApiResponse`（统一成功/错误响应格式）
//! - 应用级共享状态 `AppState`（通过 axum 的 State 提取器注入）
//! - 自定义错误类型 `AppError`（实现 `IntoResponse` 自动转为 HTTP 响应）
//! - 业务处理器示例（用户管理 CRUD）
//!
//! # 处理器层在整个架构中的位置
//!
//! ```text
//! 请求 → Server → Middleware链 → Router → [Handler]
//!                                             ↑
//!                                        （当前模块）
//! ```
//!
//! # axum 处理器的工作方式
//!
//! axum 的处理器是普通的 async 函数，通过"提取器"（Extractor）模式
//! 从请求中提取所需数据。提取器按参数顺序依次执行，例如：
//!
//! ```ignore
//! async fn create_user(
//!     State(state): State<AppState>,       // 提取共享状态
//!     Path(id): Path<String>,               // 提取路径参数
//!     Query(params): Query<SearchParams>,   // 提取查询参数
//!     Json(body): Json<CreateUserReq>,      // 提取 JSON 请求体
//! ) -> impl IntoResponse {                  // 返回值实现 IntoResponse
//!     // 业务逻辑
//! }
//! ```
//!
//! # 提取器执行顺序
//!
//! 提取器按声明顺序执行。注意：请求体只能被消费一次，
//! 因此 `Json`、`Form` 等消费 body 的提取器只能有一个，
//! 且通常放在最后。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// 导入日志宏
use crate::{log_info, log_error, log_debug};

// ============================================================
// 通用响应结构
// ============================================================

/// 统一 API 响应结构
///
/// 所有成功响应都使用此结构包装，保证响应格式一致：
/// ```json
/// { "code": 0, "message": "success", "data": {...} }
/// ```
///
/// # 类型参数
/// * `T` - data 字段的数据类型，需实现 `Serialize`
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 业务状态码（0 表示成功，非 0 表示业务错误）
    pub code: i32,
    /// 提示信息
    pub message: String,
    /// 响应数据
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应
    ///
    /// # 参数
    /// * `data` - 响应数据
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }
}

/// 为 `ApiResponse<()>`（无数据响应）定义专属方法
///
/// 由于 `success_empty` 不依赖泛型 T，将其放在独立的 impl 块中，
/// 避免 `ApiResponse::success_empty()` 调用时需要推断 T 的问题。
impl ApiResponse<()> {
    /// 创建无数据的成功响应
    pub fn success_empty() -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: None,
        }
    }
}

/// 为响应实现 IntoResponse
///
/// 任何 `ApiResponse<T>` 都可以直接作为处理器返回值，
/// axum 会自动调用 `into_response()` 转换为 HTTP 响应。
/// （泛型实现已覆盖 T=() 的情况，无需特化实现）
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

// ============================================================
// 自定义错误类型
// ============================================================

/// 应用错误类型
///
/// 实现了 `IntoResponse` trait，可以作为处理器返回值。
/// 当处理器返回 `Result<T, AppError>` 时，`Err` 分支会自动
/// 转换为对应的 HTTP 错误响应。
///
/// # 错误分类
/// - `BadRequest` (400)：客户端请求参数错误
/// - `NotFound` (404)：资源不存在
/// - `Conflict` (409)：资源冲突
/// - `Internal` (500)：服务器内部错误
#[derive(Debug)]
pub enum AppError {
    /// 请求参数错误（400）
    BadRequest(String),
    /// 资源不存在（404）
    NotFound(String),
    /// 资源冲突（409）
    Conflict(String),
    /// 服务器内部错误（500）
    Internal(String),
}

impl AppError {
    /// 获取错误对应的 HTTP 状态码
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 获取错误消息
    fn message(&self) -> &str {
        match self {
            AppError::BadRequest(m) => m,
            AppError::NotFound(m) => m,
            AppError::Conflict(m) => m,
            AppError::Internal(m) => m,
        }
    }
}

/// 实现 IntoResponse，使 AppError 可作为 HTTP 响应返回
///
/// 将错误转换为统一格式的 JSON 响应：
/// ```json
/// { "code": 1, "message": "错误描述", "data": null }
/// ```
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        // 记录错误日志
        log_error!("请求处理错误: {} - {}", status, self.message());

        let body = serde_json::json!({
            "code": status.as_u16() as i32,
            "message": self.message(),
            "data": null,
        });
        (status, Json(body)).into_response()
    }
}

/// 为 anyhow::Error 实现 Into AppError
/// 方便在处理器中用 `?` 传播错误
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

// ============================================================
// 应用共享状态
// ============================================================

/// 应用共享状态
///
/// 通过 axum 的 `State` 提取器注入到每个处理器中，
/// 用于在处理器间共享数据（如数据库连接、配置等）。
///
/// # 设计说明
/// - 使用 `Arc` 实现廉价克隆，每个请求共享同一份数据
/// - 内部可变状态使用 `Mutex` 保护（此处用 `tokio::sync::Mutex`）
///
/// # 示例
/// ```ignore
/// async fn get_users(State(state): State<AppState>) -> impl IntoResponse {
///     let store = state.user_store.lock().await;
///     let users = store.list_all();
///     ApiResponse::success(users)
/// }
/// ```
#[derive(Clone)]
pub struct AppState {
    /// 用户数据存储（示例：内存存储）
    pub user_store: Arc<Mutex<UserStore>>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        Self {
            user_store: Arc::new(Mutex::new(UserStore::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 用户数据模型与存储（示例业务）
// ============================================================

/// 用户模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户 ID
    pub id: u64,
    /// 用户名
    pub name: String,
    /// 邮箱
    pub email: String,
}

/// 创建用户请求体
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// 用户名
    pub name: String,
    /// 邮箱
    pub email: String,
}

/// 更新用户请求体
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    /// 用户名（可选）
    pub name: Option<String>,
    /// 邮箱（可选）
    pub email: Option<String>,
}

/// 用户查询参数
#[derive(Debug, Deserialize)]
pub struct UserQueryParams {
    /// 按用户名搜索（可选）
    pub name: Option<String>,
}

/// 内存用户存储（示例）
///
/// 生产环境应替换为数据库。这里用 `HashMap` 简化演示。
pub struct UserStore {
    /// 用户数据（id -> User）
    users: std::collections::HashMap<u64, User>,
    /// 自增 ID 计数器
    next_id: u64,
}

impl UserStore {
    /// 创建空的用户存储
    pub fn new() -> Self {
        Self {
            users: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    /// 创建用户
    pub fn create(&mut self, req: CreateUserRequest) -> User {
        let user = User {
            id: self.next_id,
            name: req.name,
            email: req.email,
        };
        self.users.insert(user.id, user.clone());
        self.next_id += 1;
        user
    }

    /// 根据 ID 查找用户
    pub fn find(&self, id: u64) -> Option<User> {
        self.users.get(&id).cloned()
    }

    /// 列出所有用户
    pub fn list_all(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }

    /// 更新用户
    pub fn update(&mut self, id: u64, req: UpdateUserRequest) -> Option<User> {
        let user = self.users.get_mut(&id)?;
        if let Some(name) = req.name {
            user.name = name;
        }
        if let Some(email) = req.email {
            user.email = email;
        }
        Some(user.clone())
    }

    /// 删除用户
    pub fn delete(&mut self, id: u64) -> Option<User> {
        self.users.remove(&id)
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 业务处理器实现
// ============================================================
//
// 以下处理器演示了 axum 常用提取器的用法：
// - State:   共享状态
// - Path:    路径参数（如 /users/:id 中的 id）
// - Query:   查询参数（如 ?name=xxx）
// - Json:    JSON 请求体（自动反序列化）

/// 获取用户列表
///
/// - 提取器：`State<AppState>`、`Query<UserQueryParams>`
/// - 方法：GET
/// - 路径：/api/users
///
/// 支持通过 `?name=xxx` 按用户名过滤
pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<UserQueryParams>,
) -> Result<ApiResponse<Vec<User>>, AppError> {
    log_info!("处理获取用户列表请求，过滤条件: {:?}", params.name);

    let store = state.user_store.lock().await;
    let mut users = store.list_all();

    // 如果提供了 name 参数，进行过滤
    if let Some(ref name) = params.name {
        log_debug!("按用户名过滤: {}", name);
        users.retain(|u| u.name.contains(name));
    }

    log_info!("返回 {} 个用户", users.len());
    Ok(ApiResponse::success(users))
}

/// 获取单个用户
///
/// - 提取器：`State<AppState>`、`Path<u64>`
/// - 方法：GET
/// - 路径：/api/users/:id
///
/// 路径参数 `:id` 会自动解析为 `u64`，解析失败返回 400
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<ApiResponse<User>, AppError> {
    log_info!("处理获取用户请求，ID: {}", id);

    let store = state.user_store.lock().await;
    let user = store
        .find(id)
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?;

    Ok(ApiResponse::success(user))
}

/// 创建用户
///
/// - 提取器：`State<AppState>`、`Json<CreateUserRequest>`
/// - 方法：POST
/// - 路径：/api/users
///
/// `Json` 提取器会自动反序列化请求体，格式错误返回 400
pub async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, ApiResponse<User>), AppError> {
    log_info!("处理创建用户请求: name={}, email={}", req.name, req.email);

    // 简单的输入校验
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("用户名不能为空".to_string()));
    }
    if !req.email.contains('@') {
        return Err(AppError::BadRequest("邮箱格式不正确".to_string()));
    }

    let mut store = state.user_store.lock().await;
    let user = store.create(req);
    log_info!("用户创建成功，ID: {}", user.id);

    // 返回 201 Created 状态码和用户数据
    Ok((StatusCode::CREATED, ApiResponse::success(user)))
}

/// 更新用户
///
/// - 提取器：`State<AppState>`、`Path<u64>`、`Json<UpdateUserRequest>`
/// - 方法：PUT
/// - 路径：/api/users/:id
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<ApiResponse<User>, AppError> {
    log_info!("处理更新用户请求，ID: {}", id);

    let mut store = state.user_store.lock().await;
    let user = store
        .update(id, req)
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?;

    log_info!("用户更新成功，ID: {}", id);
    Ok(ApiResponse::success(user))
}

/// 删除用户
///
/// - 提取器：`State<AppState>`、`Path<u64>`
/// - 方法：DELETE
/// - 路径：/api/users/:id
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<ApiResponse<()>, AppError> {
    log_info!("处理删除用户请求，ID: {}", id);

    let mut store = state.user_store.lock().await;
    store
        .delete(id)
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?;

    log_info!("用户删除成功，ID: {}", id);
    Ok(ApiResponse::success_empty())
}

/// 健康检查处理器
///
/// 用于负载均衡器或容器编排系统探测服务存活状态。
/// 不需要任何提取器，直接返回固定响应。
pub async fn health_check() -> impl IntoResponse {
    log_debug!("健康检查请求");
    let data: serde_json::Value = serde_json::json!({
        "status": "ok",
        "service": "learn_tauri_http",
    });
    ApiResponse::success(data)
}

/// 根路径欢迎页
pub async fn root() -> impl IntoResponse {
    let data: serde_json::Value = serde_json::json!({
        "service": "learn_tauri_http",
        "version": "0.1.0",
        "docs": "/api/users",
    });
    ApiResponse::success(data)
}
