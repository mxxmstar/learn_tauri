//! OpenGL 渲染模块
//!
//! 提供基于 OpenGL 的视频渲染能力，当前主要支持 MJPEG（Motion JPEG）格式。
//!
//! # 模块结构
//!
//! | 文件 | 作用 |
//! |------|------|
//! | `error.rs`   | 错误类型定义（OpenGL、MJPEG 解码、纹理操作等） |
//! | `shader.rs`  | 着色器编译、链接、uniform 设置 |
//! | `texture.rs` | OpenGL 纹理创建、更新、绑定 |
//! | `mjpeg.rs`   | MJPEG 流解析、JPEG→RGBA 解码 |
//! | `renderer.rs`| 完整渲染器（窗口管理 + 渲染循环） |
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use learn_tauri_lib::render::renderer::MjpegRenderer;
//!
//! // 在新线程中启动渲染器
//! let (frame_tx, handle) = MjpegRenderer::spawn(1280, 720, "MJPEG 播放器")
//!     .expect("渲染器启动失败");
//!
//! // 发送 JPEG 帧数据
//! frame_tx.send(jpeg_data).ok();
//!
//! // 等待渲染结束
//! handle.join().unwrap().unwrap();
//! ```
//!
//! # 低层级 API
//!
//! 如果不需要完整的窗口渲染，也可以单独使用各子模块：
//!
//! - `mjpeg::MjpegParser` + `mjpeg::decode_jpeg_to_rgba` 解析并解码 MJPEG 流
//! - `texture::Texture` 管理 GPU 纹理
//! - `shader::ShaderProgram` 管理着色器

pub mod error;
pub mod mjpeg;
pub mod renderer;
pub mod shader;
pub mod texture;
