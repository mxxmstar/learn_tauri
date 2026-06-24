# HTTP 客户端模块

基于 `reqwest` 库封装的异步 HTTP 客户端模块，提供统一、简洁的接口用于发送 RESTful HTTP 请求。

---

## 目录

- [功能特性](#功能特性)
- [模块结构](#模块结构)
- [快速开始](#快速开始)
- [API 参考](#api-参考)
- [完整示例](#完整示例)
- [错误处理](#错误处理)
- [高级用法](#高级用法)
- [设计说明](#设计说明)

---

## 功能特性

- **统一接口**：所有请求方法返回统一的 `HttpResponse` 封装
- **方法支持**：GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS
- **连接池**：内部维护 TCP 连接池，自动复用连接
- **超时控制**：支持自定义请求超时时间
- **自动重定向**：默认支持 HTTP 重定向追踪
- **JSON 支持**：自动序列化请求体 / 反序列化响应体
- **自定义请求头**：支持任意 HTTP 请求头设置
- **查询参数**：支持 URL 查询参数（?key=value）
- **请求日志**：自动记录每个请求的 URL、状态码、耗时和响应大小
- **回调机制**：支持注册请求完成回调，用于日志、监控等场景
- **错误分类**：精确区分构建错误、网络错误、状态码错误和解析错误

---

## 模块结构

```text
src-tauri/src/httpclient/
├── mod.rs          # 模块入口，重新导出常用类型
├── client.rs       # 核心客户端实现（HttpClient）
├── error.rs        # 错误类型定义（HttpClientError）
├── types.rs        # 数据类型定义（请求配置、响应封装、回调信息）
└── README.md       # 本文件
```

### 文件职责

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块声明、子模块组织、常用类型的重新导出 |
| `client.rs` | `HttpClient` 结构体，封装 reqwest，提供请求方法 |
| `error.rs` | `HttpClientError` 枚举，定义六类错误及转换逻辑 |
| `types.rs` | `HttpMethod`、`RequestConfig`、`HttpResponse`、`CallbackInfo` |

---

## 快速开始

### 1. 在 `lib.rs` 中注册模块

```rust
// src-tauri/src/lib.rs
pub mod httpclient;
```

### 2. 基本 GET 请求

```rust
use crate::httpclient::HttpClient;

async fn fetch_data() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new();

    // 简单 GET 请求
    let resp = client.get("https://jsonplaceholder.typicode.com/posts/1").await?;

    println!("状态码: {}", resp.status);       // 200
    println!("响应体: {}", resp.body);           // JSON 字符串

    // 判断请求是否成功
    if resp.is_success() {
        println!("请求成功！");
    }

    Ok(())
}
```

### 3. POST JSON 请求

```rust
use crate::httpclient::HttpClient;

async fn create_post() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new();

    let body = serde_json::json!({
        "title": "测试文章",
        "body": "这是文章内容",
        "userId": 1
    });

    let resp = client.post_json("https://jsonplaceholder.typicode.com/posts", body).await?;

    println!("创建成功，ID: {}", resp.status);
    println!("返回数据: {}", resp.body);

    Ok(())
}
```

---

## API 参考

### HttpClient

```rust
// 创建客户端
let client = HttpClient::new();

// 创建客户端（自定义 reqwest 配置）
use reqwest::ClientBuilder;
let builder = ClientBuilder::new().danger_accept_invalid_certs(true);
let client = HttpClient::with_builder(builder)?;

// 注册完成回调
let client = HttpClient::new().on_complete(|info| {
    println!("{} {} -> {} ({}ms)", info.method, info.url, info.status, info.duration_ms);
});
```

### 请求方法

| 方法 | 说明 |
|------|------|
| `get(url)` | 发送 GET 请求 |
| `post_json(url, body)` | 发送 POST 请求，自动设置 Content-Type: application/json |
| `put_json(url, body)` | 发送 PUT 请求，自动设置 Content-Type: application/json |
| `delete(url)` | 发送 DELETE 请求 |
| `send(config)` | 通用请求方法，接受 `RequestConfig` |
| `send_expect_success(config)` | 发送请求并断言返回成功状态码（200-299） |

### RequestConfig（建造者模式）

```rust
use crate::httpclient::{HttpMethod, RequestConfig};

let config = RequestConfig::new("https://api.example.com/users")
    .method(HttpMethod::POST)                       // 设置请求方法
    .header("Authorization", "Bearer token123")      // 设置请求头
    .header("X-Custom-Header", "custom_value")
    .query("page", "1")                              // 设置查询参数
    .query("limit", "10")
    .json(serde_json::json!({"name": "张三"}))        // 设置 JSON 请求体
    .timeout_secs(60);                                // 设置超时时间（秒）

let resp = client.send(config).await?;
```

### HttpResponse

| 字段/方法 | 类型 | 说明 |
|-----------|------|------|
| `status` | `u16` | HTTP 状态码 |
| `headers` | `HashMap<String, String>` | 响应头 |
| `body` | `String` | 响应体原始文本 |
| `json::<T>()` | `Result<T, HttpClientError>` | 将响应体解析为指定 JSON 类型 |
| `is_success()` | `bool` | 是否 200-299 |
| `is_client_error()` | `bool` | 是否 400-499 |
| `is_server_error()` | `bool` | 是否 500-599 |

---

## 完整示例

### 结合 Tauri 命令使用

```rust
// src-tauri/src/lib.rs

use crate::httpclient::{HttpClient, HttpClientError, HttpResponse};

#[tauri::command]
async fn fetch_user(user_id: u32) -> Result<HttpResponse, String> {
    let client = HttpClient::new();
    let url = format!("https://jsonplaceholder.typicode.com/users/{}", user_id);

    client.get(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_user(name: String, email: String) -> Result<HttpResponse, String> {
    let client = HttpClient::new();
    let body = serde_json::json!({ "name": name, "email": email });

    client.post_json("https://jsonplaceholder.typicode.com/users", body)
        .await
        .map_err(|e| e.to_string())
}
```

### 解析 JSON 响应

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Post {
    userId: u32,
    id: u32,
    title: String,
    body: String,
}

async fn get_post() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new();
    let resp = client.get("https://jsonplaceholder.typicode.com/posts/1").await?;

    // 将响应体解析为结构体
    let post: Post = resp.json()?;
    println!("标题: {}，内容: {}", post.title, post.body);

    Ok(())
}
```

---

## 错误处理

`HttpClientError` 枚举定义了六类错误，覆盖 HTTP 请求的全生命周期：

| 错误类型 | 触发场景 | 示例 |
|----------|----------|------|
| `RequestBuildError` | 请求构建失败 | URL 格式错误、请求头不合法 |
| `RequestError` | 网络请求失败 | DNS 解析失败、连接被拒绝 |
| `HttpStatusError` | 服务端返回错误状态码 | 404 Not Found、500 Internal Server Error |
| `ResponseParseError` | 响应体解析失败 | JSON 格式错误 |
| `UrlParseError` | URL 格式错误 | 不完整的 URL |
| `TimeoutError` | 请求超时 | 服务端响应超过设定的超时时间 |
| `TlsError` | TLS/SSL 错误 | 证书验证失败 |

### 错误处理示例

```rust
use crate::httpclient::HttpClientError;

async fn safe_request() -> Result<(), HttpClientError> {
    let client = HttpClient::new();
    let result = client.get("https://api.example.com/data").await;

    match result {
        Ok(resp) => {
            println!("成功: {}", resp.body);
            Ok(())
        }
        Err(e) => {
            // 根据错误类型分别处理
            match &e {
                HttpClientError::TimeoutError(msg) => {
                    eprintln!("请求超时，请检查网络: {}", msg);
                }
                HttpClientError::HttpStatusError { status, body } => {
                    eprintln!("服务端错误 {}: {}", status, body);
                }
                _ => {
                    eprintln!("请求失败: {}", e);
                }
            }
            Err(e)
        }
    }
}
```

---

## 高级用法

### 自定义客户端配置

```rust
use reqwest::ClientBuilder;
use crate::httpclient::HttpClient;

// 创建带有自定义 TLS 配置的客户端
let builder = ClientBuilder::new()
    .timeout(std::time::Duration::from_secs(60))   // 全局超时
    .connect_timeout(std::time::Duration::from_secs(10))  // 连接超时
    .danger_accept_invalid_certs(true)              // 跳过证书验证（仅开发环境）
    .no_proxy();                                    // 不使用代理

let client = HttpClient::with_builder(builder)?;
```

### 使用 RequestConfig 精细控制

```rust
use crate::httpclient::{HttpMethod, RequestConfig};

let config = RequestConfig::new("https://api.example.com/search")
    .method(HttpMethod::GET)
    .header("Authorization", "Bearer token123")
    .query("q", "rust http client")
    .query("page", "1")
    .query("limit", "20")
    .timeout_secs(15);

let resp = client.send(config).await?;
```

### 带回调的监控

```rust
use std::time::Instant;
use crate::httpclient::HttpClient;

let client = HttpClient::new().on_complete(|info| {
    // 可以将此信息发送到监控系统
    println!(
        "[HTTP] {} {} -> {} ({}ms, {} bytes)",
        info.method, info.url, info.status, info.duration_ms, info.body_size
    );
});
```

---

## 设计说明

### 为什么用 reqwest？

| 特性 | 说明 |
|------|------|
| **社区标准** | reqwest 是 Rust 生态中最广泛使用的 HTTP 客户端库 |
| **异步原生** | 基于 tokio 异步运行时，不会阻塞主线程 |
| **连接池** | 内置连接池自动复用 TCP 连接，减少握手开销 |
| **自动重定向** | 默认支持 HTTP 重定向追踪（最多 10 次） |
| **TLS 支持** | 通过 `native-tls` 或 `rustls` 提供 HTTPS 支持 |
| **JSON 处理** | 与 serde 深度集成，支持自动序列化/反序列化 |
| **流式请求** | 支持请求体/响应体的流式读写 |

### 架构模式

采用**封装模式**（Wrapper Pattern），在 reqwest 之上提供更简洁的接口：

```text
┌─────────────────────────────────────────────┐
│              调用方代码                         │
│    (Tauri 命令 / 业务模块 / 测试代码)           │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│         HttpClient（本模块）                   │
│  ┌─────────────────────────────────────────┐ │
│  │  get() / post_json() / send() ...        │ │
│  │  统一返回 Result<HttpResponse, Error>     │ │
│  │  自动日志 + 可选回调                       │ │
│  └──────────────────┬──────────────────────┘ │
└─────────────────────┬────────────────────────┘
                      │
┌─────────────────────▼────────────────────────┐
│            reqwest::Client                     │
│    （连接池 / 自动重定向 / TLS / 超时）         │
└──────────────────────────────────────────────┘
```

### 与 HTTP 服务模块（`http/`）的关系

本项目的 `http/` 模块是基于 axum 的 **HTTP 服务端**实现，用于提供 Web API 服务；
而 `httpclient/` 模块是基于 reqwest 的 **HTTP 客户端**实现，用于向外部服务发起请求。
两者相辅相成，分别负责"提供服务"和"消费服务"，共同组成完整的 HTTP 通信能力。

| 对比维度 | `http/` 模块 | `httpclient/` 模块 |
|----------|-------------|-------------------|
| 角色 | 服务端（Server） | 客户端（Client） |
| 基础库 | axum | reqwest |
| 功能 | 接收并处理请求 | 发送并获取响应 |
| 典型场景 | 提供 RESTful API | 调用第三方 API |
