# HTTP 服务模块使用说明

本模块基于 [axum](https://github.com/tokio-rs/axum) 框架实现，采用 **handler → middleware → router → server** 分层架构。

## 目录

- [架构总览](#架构总览)
- [模块结构](#模块结构)
- [各层职责详解](#各层职责详解)
- [快速开始](#快速开始)
- [添加自定义处理器](#添加自定义处理器)
- [添加自定义中间件](#添加自定义中间件)
- [添加自定义路由](#添加自定义路由)
- [应用共享状态](#应用共享状态)
- [统一响应与错误处理](#统一响应与错误处理)
- [API 接口文档](#api-接口文档)
- [常见问题](#常见问题)

---

## 架构总览

```text
┌─────────────────────────────────────────────────────────┐
│                    客户端请求                            │
└────────────────────────┬────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Server 层 (server.rs)                                  │
│  - TCP 监听与连接管理                                    │
│  - 优雅关闭（Ctrl+C）                                    │
│  - axum::serve 启动                                      │
└────────────────────────┬────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Middleware 层 (middleware.rs)                          │
│  - 日志中间件（记录请求/响应）                           │
│  - 请求 ID 中间件（生成唯一 ID）                         │
│  - 计时中间件（记录耗时）                                │
│  - CORS 中间件（tower-http）                            │
│  - Trace 中间件（tower-http）                           │
└────────────────────────┬────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Router 层 (router.rs)                                  │
│  - 路由注册（GET/POST/PUT/DELETE）                       │
│  - 路由分组（nest 嵌套）                                 │
│  - 路径参数提取（:id）                                   │
│  - 中间件作用域管理                                      │
└────────────────────────┬────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Handler 层 (handler.rs)                                │
│  - 业务逻辑处理                                          │
│  - 请求提取器（State/Path/Query/Json）                  │
│  - 响应构造（ApiResponse/AppError）                      │
│  - 数据模型与存储                                        │
└─────────────────────────────────────────────────────────┘
```

### 请求处理流程

```text
1. TCP 连接接入（Server 层）
2. axum 解析 HTTP 请求
3. 执行中间件链（Middleware 层，洋葱模型）
4. 路由匹配（Router 层）
5. 调用处理器（Handler 层）
6. 构造响应并返回
```

---

## 模块结构

```
http/
├── mod.rs          # 模块入口，定义 HttpService 门面
├── server.rs       # 服务器层：TCP 监听、优雅关闭
├── middleware.rs   # 中间件层：日志、请求 ID、计时
├── router.rs       # 路由层：路由注册、分组、中间件组装
└── handler.rs      # 处理器层：业务逻辑、提取器、响应
```

---

## 各层职责详解

### 1. Server 层 (`server.rs`)

**职责**：管理服务器生命周期，绑定 TCP 端口，启动 axum 服务。

**核心类型**：
- `HttpServer`：HTTP 服务器封装
- `ServerConfig`：服务器配置（host、port、优雅关闭）

**核心方法**：
| 方法 | 说明 |
|------|------|
| `HttpServer::new(addr, state)` | 创建服务器实例 |
| `HttpServer::with_config(config)` | 自定义配置 |
| `HttpServer::serve().await` | 启动服务器（阻塞） |

### 2. Middleware 层 (`middleware.rs`)

**职责**：在请求到达处理器前/后执行通用逻辑（日志、鉴权、CORS 等）。

**中间件清单**：
| 中间件 | 功能 |
|--------|------|
| `logging_middleware` | 记录请求方法、路径、状态码、耗时 |
| `request_id_middleware` | 生成/复用请求 ID，写入响应头 |
| `timing_middleware` | 记录耗时到 `X-Response-Time` 响应头 |

**执行顺序（洋葱模型）**：
```text
请求 → 中间件A（前置）
  请求 → 中间件B（前置）
    请求 → Handler → 响应
  响应 ← 中间件B（后置）
响应 ← 中间件A（后置）
```

### 3. Router 层 (`router.rs`)

**职责**：注册路由、路由分组、组装中间件。

**核心函数**：
| 函数 | 说明 |
|------|------|
| `build_app_router(state)` | 构建应用主路由器（总入口） |
| `build_user_routes()` | 构建用户管理子路由 |

**路由模式**：
| 模式 | 示例 | 说明 |
|------|------|------|
| 静态路由 | `/api/users` | 精确匹配 |
| 参数路由 | `/api/users/:id` | `:id` 捕获路径段 |
| 通配符路由 | `/files/*path` | `*path` 捕获剩余路径 |

### 4. Handler 层 (`handler.rs`)

**职责**：实现具体业务逻辑，使用提取器获取请求数据。

**核心类型**：
| 类型 | 说明 |
|------|------|
| `AppState` | 应用共享状态 |
| `ApiResponse<T>` | 统一成功响应结构 |
| `AppError` | 统一错误类型（自动转 HTTP 响应） |
| `User` / `UserStore` | 用户数据模型与存储 |

**提取器**（按声明顺序执行）：
| 提取器 | 用途 | 示例 |
|--------|------|------|
| `State<T>` | 共享状态 | `State(state): State<AppState>` |
| `Path<T>` | 路径参数 | `Path(id): Path<u64>` |
| `Query<T>` | 查询参数 | `Query(params): Query<UserQueryParams>` |
| `Json<T>` | JSON 请求体 | `Json(req): Json<CreateUserRequest>` |

---

## 快速开始

### 最简启动

```rust
use learn_tauri_lib::http::HttpService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建并启动 HTTP 服务，监听 0.0.0.0:8080
    let service = HttpService::new("0.0.0.0:8080");
    service.start().await?;
    Ok(())
}
```

启动后可访问：
- `http://localhost:8080/` - 服务信息
- `http://localhost:8080/health` - 健康检查
- `http://localhost:8080/api/users` - 用户列表

### 在 Tauri 应用中后台启动

```rust
use learn_tauri_lib::http::HttpService;

pub fn run() {
    // ... 日志初始化 ...

    tauri::Builder::default()
        .setup(|_app| {
            // 在后台线程启动 HTTP 服务
            tokio::spawn(async {
                let service = HttpService::new("0.0.0.0:8080");
                if let Err(e) = service.start().await {
                    eprintln!("HTTP 服务启动失败: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 添加自定义处理器

处理器是普通的 async 函数，通过提取器获取请求数据。

### 示例：添加一个返回当前时间的处理器

**第 1 步**：在 `handler.rs` 中添加处理器函数

```rust
use axum::extract::State;
use crate::http::handler::{AppState, ApiResponse, AppError};

/// 获取当前时间
pub async fn get_time() -> impl IntoResponse {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let data: serde_json::Value = serde_json::json!({
        "timestamp": ts,
        "unit": "seconds",
    });
    ApiResponse::success(data)
}
```

**第 2 步**：在 `router.rs` 中注册路由

```rust
// 在 build_app_router 中添加
.route("/time", get(crate::http::handler::get_time))
```

---

## 添加自定义中间件

中间件是签名 `async fn(Request, Next) -> Response` 的函数。

### 示例：添加鉴权中间件

**第 1 步**：在 `middleware.rs` 中添加

```rust
use axum::{extract::Request, middleware::Next, response::Response};
use axum::http::{StatusCode, HeaderValue};

/// 简单的 Token 鉴权中间件
pub async fn auth_middleware(req: Request, next: Next) -> Response {
    // 检查 Authorization 请求头
    let auth_header = req.headers().get("authorization");

    match auth_header.and_then(|v| v.to_str().ok()) {
        Some(token) if token == "Bearer secret-token" => {
            // 鉴权通过，继续处理
            next.run(req).await
        }
        _ => {
            // 鉴权失败，返回 401
            let body = serde_json::json!({
                "code": 401,
                "message": "未授权",
                "data": null,
            });
            (StatusCode::UNAUTHORIZED, Json(body)).into_response()
        }
    }
}
```

**第 2 步**：在 `router.rs` 中应用

```rust
use axum::middleware::from_fn;
use crate::http::middleware::auth_middleware;

// 全局应用（对所有路由生效）
.layer(from_fn(auth_middleware))

// 或仅对特定路由组应用
let protected = Router::new()
    .route("/admin", get(admin_handler))
    .layer(from_fn(auth_middleware));
```

### 中间件执行顺序

> **重要**：axum 中 `layer()` 的添加顺序与执行顺序**相反**。
> 先添加的 layer 后执行（最外层），后添加的 layer 先执行（最内层）。

```rust
Router::new()
    .route("/", get(handler))
    .layer(from_fn(middleware_a))   // 后执行（最外层）
    .layer(from_fn(middleware_b));  // 先执行（最内层）

// 执行顺序：middleware_b → handler → middleware_a 后置
```

---

## 添加自定义路由

### 1. 添加单个路由

在 `router.rs` 的 `build_app_router` 中添加：

```rust
.route("/hello", get(hello_handler))
.route("/echo", post(echo_handler))
```

### 2. 添加路由组（嵌套路由）

使用 `nest` 实现路由分组，便于模块化管理：

```rust
// 在 router.rs 中
fn build_order_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_orders))
        .route("/:id", get(get_order))
}

// 在 build_app_router 中嵌套
.nest("/api/orders", build_order_routes())
```

### 3. 同一路径绑定多个方法

```rust
.route(
    "/api/users/:id",
    get(get_user)         // GET /api/users/:id
        .put(update_user) // PUT /api/users/:id
        .delete(delete_user), // DELETE /api/users/:id
)
```

---

## 应用共享状态

`AppState` 通过 axum 的 `State` 提取器注入到每个处理器，用于共享数据。

### 1. 扩展 AppState

```rust
// handler.rs
#[derive(Clone)]
pub struct AppState {
    pub user_store: Arc<Mutex<UserStore>>,
    pub db_pool: Arc<sqlx::PgPool>,  // 新增：数据库连接池
    pub config: Arc<Config>,          // 新增：应用配置
}

impl AppState {
    pub fn new() -> Self {
        Self {
            user_store: Arc::new(Mutex::new(UserStore::new())),
            db_pool: Arc::new(/* 初始化连接池 */),
            config: Arc::new(/* 加载配置 */),
        }
    }
}
```

### 2. 在处理器中使用

```rust
pub async fn get_db_version(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let version = state.db_pool.execute("SELECT version()").await?;
    ApiResponse::success(serde_json::json!({ "version": version }))
}
```

### 3. 传入自定义状态启动

```rust
let state = AppState::new();
let service = HttpService::with_state("0.0.0.0:8080", state);
service.start().await?;
```

---

## 统一响应与错误处理

### 成功响应

所有成功响应使用 `ApiResponse<T>` 包装，保证格式统一：

```json
{
  "code": 0,
  "message": "success",
  "data": { ... }
}
```

```rust
// 返回数据
Ok(ApiResponse::success(users))

// 无数据返回
Ok(ApiResponse::success_empty())

// 自定义状态码（如 201 Created）
Ok((StatusCode::CREATED, ApiResponse::success(new_user)))
```

### 错误响应

使用 `AppError` 枚举，自动转换为对应的 HTTP 状态码：

```rust
pub enum AppError {
    BadRequest(String),    // 400
    NotFound(String),      // 404
    Conflict(String),      // 409
    Internal(String),      // 500
}
```

```rust
// 在处理器中返回错误
Ok(store.find(id)
    .ok_or_else(|| AppError::NotFound(format!("用户 {} 不存在", id)))?)
```

错误响应格式：

```json
{
  "code": 404,
  "message": "用户 1 不存在",
  "data": null
}
```

---

## API 接口文档

### 用户管理 API

| 方法 | 路径 | 说明 | 请求体 | 响应 |
|------|------|------|--------|------|
| GET | `/` | 服务信息 | - | 服务元数据 |
| GET | `/health` | 健康检查 | - | `{"status":"ok"}` |
| GET | `/api/users` | 用户列表 | - | `User[]` |
| GET | `/api/users` | 用户列表（过滤） | -（query: `?name=xxx`） | `User[]` |
| GET | `/api/users/:id` | 获取单个用户 | - | `User` |
| POST | `/api/users/create` | 创建用户 | `CreateUserRequest` | `User` (201) |
| PUT | `/api/users/:id` | 更新用户 | `UpdateUserRequest` | `User` |
| DELETE | `/api/users/:id` | 删除用户 | - | `()` |

### 请求/响应示例

**创建用户**：
```bash
curl -X POST http://localhost:8080/api/users/create \
  -H "Content-Type: application/json" \
  -d '{"name":"张三","email":"zhangsan@example.com"}'
```

响应（201）：
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "id": 1,
    "name": "张三",
    "email": "zhangsan@example.com"
  }
}
```

**获取用户**：
```bash
curl http://localhost:8080/api/users/1
```

**过滤用户**：
```bash
curl "http://localhost:8080/api/users?name=张"
```

---

## 常见问题

### Q: 如何修改监听端口？

```rust
let service = HttpService::new("0.0.0.0:3000"); // 监听 3000 端口
```

或使用配置：

```rust
use learn_tauri_lib::http::{HttpServer, ServerConfig, AppState};

let config = ServerConfig {
    host: "127.0.0.1".to_string(),
    port: 9000,
    graceful_shutdown: true,
};
let server = HttpServer::new("127.0.0.1:9000", AppState::new()).with_config(config);
server.serve().await?;
```

### Q: 中间件为什么执行顺序和添加顺序相反？

这是 axum/tower 的设计。`layer()` 会"包裹"在当前 Router 外层。
先添加的 layer 在更外层，因此请求先穿过内层（后添加的）再到达外层。

如需控制顺序，可使用 `ServiceBuilder` 显式编排：

```rust
use tower::ServiceBuilder;

Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            .layer(from_fn(middleware_a))  // 先执行
            .layer(from_fn(middleware_b))  // 后执行
    );
```

### Q: 如何处理请求体不是 JSON 的情况？

使用 `axum::body::Bytes` 或 `String` 提取器：

```rust
use axum::body::Bytes;

async fn upload(Bytes(data): Bytes) -> impl IntoResponse {
    // data 是原始字节数据
    ApiResponse::success(serde_json::json!({ "size": data.len() }))
}
```

### Q: 如何支持静态文件服务？

使用 `tower-http` 的 `ServeDir`：

```rust
use tower_http::services::ServeDir;

// 在 build_app_router 中添加
.nest_service("/static", ServeDir::new("public"))
```

### Q: 优雅关闭是如何工作的？

服务器收到 `Ctrl+C`（SIGINT）信号后：
1. 停止接受新连接
2. 等待正在处理的请求完成
3. 关闭服务器

可通过 `ServerConfig.graceful_shutdown` 配置是否启用。

---

## 扩展建议

1. **数据库集成**：替换 `UserStore` 为真实数据库（推荐 `sqlx`）
2. **JWT 认证**：在中间件层实现 JWT token 验证
3. **请求限流**：使用 `tower::limit` 实现速率限制
4. **API 文档**：集成 `utoipa` 自动生成 OpenAPI 文档
5. **WebSocket**：使用 axum 的 WebSocket 支持实时通信
6. **模板渲染**：集成 `askama` 实现 SSR 服务端渲染
7. **请求压缩**：使用 `tower-http::compression` 压缩响应
