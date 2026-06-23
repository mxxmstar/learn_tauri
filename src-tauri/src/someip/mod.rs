//! SomeIP 协议模块
//!
//! 本模块实现 SomeIP（Scalable service-Oriented MiddlewarE over IP）协议，
//! 用于车载以太网通信。
//!
//! # 模块组织
//!
//! - `method` - `SomeIPMethod` 枚举（稳定接口契约）
//! - `header` - `SomeIPHeader` 结构体（16 字节固定头）
//! - `entry` - `SomeIPEntry` 结构体（服务发现条目）
//! - `error` - `SomeIPError` 错误类型
//! - `util` - IP/MAC 地址工具函数
//! - `payload` - Payload trait + 所有 Payload 实现
//! - `message` - `SomeIPMessage` 消息构建器
//!
//! # 使用示例
//!
//! ```rust
//! # use crate::someip::message::SomeIPMessage;
//! # use crate::someip::payload::MediaPayload;
//! // 创建消息构建器
//! let mut msg = SomeIPMessage::new(0x433F, 0x0001, 0x0001, 0x01, 0x00);
//!
//! // 构建 SetMedia 请求
//! let payload = MediaPayload::default_payload();
//! let bytes = msg.build(&payload);
//!
//! // 解析响应
//! let (header, payload_bytes) = SomeIPMessage::parse(&bytes).unwrap();
//! ```
//!
//! # 设计理念
//!
//! 本模块用 Rust 的 trait 体系替代 C++ 的基类/子类继承模式：
//!
//! - `SomeIPMethod` 枚举 = 稳定接口契约（不可变）
//! - `Payload` trait = C++ 纯虚基类（可扩展）
//! - `PayloadCodec` trait = 扩展解码能力（可扩展）
//! - 具体 Payload 结构体 = C++ 子类（可替换）
//!
//! 协议变更时只需：
//! 1. 新建结构体（如 `MediaPayloadV2`）
//! 2. 实现 `Payload` / `PayloadCodec` trait
//! 3. 注册到 `PayloadRegistry`
//!
//! 无需修改现有代码（开放-封闭原则）。

// 模块声明
pub mod method;
pub mod header;
pub mod entry;
pub mod error;
pub mod util;
pub mod payload;
pub mod message;

// Re-export 常用类型
pub use method::SomeIPMethod;
pub use header::SomeIPHeader;
pub use entry::{SomeIPEntry, SomeIPStatus};
pub use error::{SomeIPError, SomeIPResult};
pub use message::SomeIPMessage;

// Payload trait 和类型
pub use payload::{Payload, PayloadCodec, EmptyPayload};
pub use payload::{
    MediaPayload, SubscribePayload, SetNetworkPayload,
    SetCameraROIPayload, GetCameraROIPayload,
    AlgROIPayload, CamExclusivePayload,
    FindOrOfferPayload, ConfigPayload,
};
pub use payload::init_default_registry;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_integration() {
        // 集成测试：验证模块能正确协同工作
        use super::*;

        let mut msg = SomeIPMessage::new(0x433F, 0x0001, 0x0001, 0x01, 0x00);
        let payload = MediaPayload::default_payload();
        let bytes = msg.build(&payload);

        assert!(!bytes.is_empty());
    }
}
