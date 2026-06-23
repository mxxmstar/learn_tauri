//! TCP 消息帧化编解码模块
//!
//! 该模块基于 `tokio_util::codec::LengthDelimitedCodec` 实现 TCP 消息的帧化，
//! 解决 TCP 流式协议的**粘包**和**半包**问题。
//!
//! # 为什么需要帧化？
//!
//! TCP 是字节流协议，没有消息边界。例如发送两条消息：
//!
//! ```text
//! 发送端: [消息A][消息B]
//! 接收端可能收到:
//!   情况1: [消息A][消息B]        ← 正常
//!   情况2: [消息A的前半][消息A的后半+消息B]  ← 粘包+半包
//!   情况3: [消息A+消息B的前半][消息B的后半]  ← 粘包+半包
//! ```
//!
//! 帧化通过在每条消息前添加**长度前缀**，让接收端能准确切分消息：
//!
//! ```text
//! 字节流: [len=10][消息A 10字节][len=5][消息B 5字节]
//!                  ↑ 第1帧 ↑          ↑ 第2帧 ↑
//! ```
//!
//! # LengthDelimitedCodec 工作原理
//!
//! `tokio_util::codec::LengthDelimitedCodec` 是 tokio 官方提供的帧化编解码器：
//!
//! - **编码**：在消息前添加 4 字节大端长度头
//! - **解码**：读取长度头，等待完整消息后输出一个 `Bytes` 帧
//! - 内部自动处理半包（缓冲等待）和粘包（切分多个帧）
//!
//! 配合 `Framed<TcpStream, LengthDelimitedCodec>` 使用时，
//! `Framed` 实现了 `Stream`（读）和 `Sink`（写）trait，
//! 可以像处理消息队列一样处理 TCP 连接。
//!
//! # 帧格式
//!
//! 本模块使用默认配置：
//!
//! ```text
//! ┌─────────────────┬──────────────────────────┐
//! │  长度头 (4 字节) │   消息体 (最多 1 MB)      │
//! │  大端 u32        │   (JSON 编码的 TcpMessage) │
//! └─────────────────┴──────────────────────────┘
//! ```
//!
//! # 使用方式
//!
//! 通常不直接使用本模块，而是通过 `TcpServer` 和 `TcpClient` 间接使用。
//! 如果需要自定义帧化行为，可参考本模块配置。

use tokio_util::codec::LengthDelimitedCodec;

/// 最大帧长度（1 MB）
///
/// 防止恶意客户端发送超大帧导致内存耗尽。
/// 如需传输更大消息，请修改此常量。
pub const MAX_FRAME_LENGTH: usize = 1024 * 1024;

/// 长度头字节数（4 字节大端 u32）
pub const LENGTH_FIELD_LENGTH: usize = 4;

/// 创建配置好的长度前缀编解码器
///
/// 返回的 codec 可用于构建 `Framed<TcpStream, LengthDelimitedCodec>`。
///
/// # 配置说明
///
/// - 长度头：4 字节大端序
/// - 最大帧长：1 MB（防止内存溢出）
/// - 长度调整：0（长度头表示的是消息体长度）
///
/// # 返回值
///
/// 返回配置好的 `LengthDelimitedCodec` 实例
pub fn new_codec() -> LengthDelimitedCodec {
    // 使用 builder 模式配置编解码器
    // tokio-util 0.7 中，max_frame_length 需要通过 builder 设置
    LengthDelimitedCodec::builder()
        .length_field_length(LENGTH_FIELD_LENGTH)
        .max_frame_length(MAX_FRAME_LENGTH)
        .new_codec()
}
