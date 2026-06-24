//! HTTP 客户端模块
//!
//! 基于 `reqwest` 库封装了功能完善的异步 HTTP 客户端，
//! 提供统一的接口用于发送 RESTful HTTP 请求。
//!
//! # 模块结构
//!
//! ```text
//! httpclient/
//! ├── mod.rs          # 模块入口，重新导出常用类型
//! ├── client.rs       # 核心客户端实现（HttpClient）
//! ├── error.rs        # 错误类型定义（HttpClientError）
//! └── types.rs        # 数据类型定义（请求配置、响应封装、回调信息等）
//! ```
//!
//! # 快速开始
//!
//! ```ignore
//! use crate::httpclient::HttpClient;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = HttpClient::new();
//!
//!     // GET 请求
//!     let resp = client.get("https://jsonplaceholder.typicode.com/posts/1").await?;
//!     println!("状态码: {}", resp.status);
//!     println!("响应体: {}", resp.body);
//!
//!     // POST 请求（JSON）
//!     let body = serde_json::json!({
//!         "title": "测试文章",
//!         "body": "这是文章内容",
//!         "userId": 1
//!     });
//!     let resp = client.post_json("https://jsonplaceholder.typicode.com/posts", body).await?;
//!     println!("创建成功，返回: {}", resp.body);
//!
//!     Ok(())
//! }
//! ```
//!
//! # 设计要点
//!
//! ## 1. 为什么用 reqwest？
//!
//! - **社区标准**：Rust 生态中最流行的 HTTP 客户端库
//! - **异步原生**：基于 tokio 异步运行时，无阻塞
//! - **连接池**：内置连接池，自动复用 TCP 连接
//! - **自动重定向**：默认支持 HTTP 重定向追踪
//! - **TLS 支持**：内置对 HTTPS 的支持
//! - **JSON 处理**：与 serde 深度集成，自动序列化/反序列化
//!
//! ## 2. 架构设计
//!
//! 采用"封装模式"，在 reqwest 之上提供更简洁的接口：
//! - **屏蔽细节**：隐藏 reqwest 的复杂配置细节
//! - **统一返回值**：所有请求方法返回 `Result<HttpResponse, HttpClientError>`
//! - **易于扩展**：通过 `RequestConfig` 建造者模式灵活配置请求
//! - **可观测性**：内置请求日志和可选的完成回调
//!
//! ## 3. 错误处理
//!
//! 错误处理覆盖 HTTP 请求的各个阶段：
//! - 构建阶段：URL 格式错误、请求头不合法
//! - 连接阶段：DNS 解析失败、连接被拒绝、超时
//! - 响应阶段：非成功状态码（4xx/5xx）
//! - 解析阶段：响应体 JSON 解析失败

pub mod client;
pub mod error;
pub mod types;

// 重新导出常用类型，方便外部使用
pub use client::HttpClient;
pub use error::HttpClientError;
pub use types::{CallbackInfo, HttpMethod, HttpResponse, RequestConfig};
