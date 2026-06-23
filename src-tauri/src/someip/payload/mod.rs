//! Payload trait 定义及所有 Payload 实现
//!
//! 本模块定义 `Payload` 和 `PayloadCodec` trait，替代 C++ 的基类/子类继承模式。
//!
//! # 设计说明
//!
//! C++ 中 `SomeIPMessage` 是上帝类，用大 `switch` 按方法 ID 分发处理逻辑。
//! Rust 中用 trait 多态替代：
//!
//! - `Payload` trait：对象安全，可用于 `Box<dyn Payload>` 动态分发。对应 C++ 各 payload 的 `ToByteArray()` 虚函数。
//! - `PayloadCodec` trait：在 `Payload` 基础上增加 `decode()` 关联函数（需要 `Sized`），用于类型安全的反序列化。对应 C++ 的 `parseXxxPayload()` 静态方法。
//!
//! # 模块组织
//!
//! - `mod.rs` - trait 定义 + `EmptyPayload` + re-exports
//! - `media.rs` - `MediaPayload`
//! - `subscribe.rs` - `SubscribePayload`
//! - `network.rs` - `SetNetworkPayload`
//! - `camera_roi.rs` - `SetCameraROIPayload` + `GetCameraROIPayload`
//! - `alg_roi.rs` - `AlgROIPayload`
//! - `cam_exclusive.rs` - `CamExclusivePayload`
//! - `find_offer.rs` - `FindOrOfferPayload`
//! - `config.rs` - `ConfigPayload`

use std::collections::HashMap;
use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;

pub mod media;
pub mod subscribe;
pub mod network;
pub mod camera_roi;
pub mod alg_roi;
pub mod cam_exclusive;
pub mod find_offer;
pub mod config;

// Re-export 所有 payload 类型
pub use media::MediaPayload;
pub use subscribe::SubscribePayload;
pub use network::SetNetworkPayload;
pub use camera_roi::{SetCameraROIPayload, GetCameraROIPayload};
pub use alg_roi::AlgROIPayload;
pub use cam_exclusive::CamExclusivePayload;
pub use find_offer::FindOrOfferPayload;
pub use config::ConfigPayload;

/// Payload 编码 trait（对象安全，可用于 `Box<dyn Payload>`）。
///
/// 对应 C++ 中各 payload 结构体的 `ToByteArray()` 虚函数。
///
/// # 设计要点
///
/// - `method_id()` 返回方法 ID（稳定接口），用于消息构建时设置 header。
/// - `encode()` 返回大端序字节序列，用于拼接在 header 后。
/// - `Send + Sync + Debug` 约束确保可用于多线程和日志。
pub trait Payload: Send + Sync + std::fmt::Debug {
    /// 返回对应的方法 ID（稳定接口）。
    ///
    /// 对应 C++ 中 `SomeIPMethod` 枚举值。
    fn method_id(&self) -> SomeIPMethod;

    /// 编码为字节序列（大端序）。
    ///
    /// 对应 C++ 中各 payload 的 `ToByteArray()` 方法。
    fn encode(&self) -> Vec<u8>;
}

/// Payload 编解码 trait（增加解码能力）。
///
/// 对应 C++ 的 `parseXxxPayload()` 静态方法。
///
/// # 设计要点
///
/// - 需要 `Sized`，不可用于 trait object（`Box<dyn PayloadCodec>` 不行）。
/// - 用于需要类型安全的反序列化场景。
/// - 实现此 trait 的类型自动实现 `Payload`。
pub trait PayloadCodec: Payload + Sized {
    /// 从字节序列解码。
    ///
    /// 对应 C++ 的 `parseXxxPayload()` 静态方法。
    fn decode(data: &[u8]) -> SomeIPResult<Self>;
}

/// 空 Payload，用于 Get 类请求（无 payload，length=0）。
///
/// 对应 C++ `Build()` 中 `SetLength(0)` 的分支。
///
/// # 使用示例
///
/// ```rust
/// # use crate::someip::payload::{EmptyPayload, Payload};
/// # use crate::someip::method::SomeIPMethod;
/// let payload = EmptyPayload::new(SomeIPMethod::GetMedia);
/// assert!(payload.encode().is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct EmptyPayload {
    /// 方法 ID
    pub method: SomeIPMethod,
}

impl EmptyPayload {
    /// 创建新的空 Payload。
    pub fn new(method: SomeIPMethod) -> Self {
        EmptyPayload { method }
    }
}

impl Payload for EmptyPayload {
    fn method_id(&self) -> SomeIPMethod {
        self.method
    }

    fn encode(&self) -> Vec<u8> {
        Vec::new()
    }
}

impl PayloadCodec for EmptyPayload {
    fn decode(_data: &[u8]) -> SomeIPResult<Self> {
        Ok(Self::default())
    }
}

/// Payload 解码器函数类型。
///
/// 对应 C++ 中 `parseXxxPayload()` 的函数指针。
type PayloadDecoder = fn(data: &[u8]) -> SomeIPResult<Box<dyn Payload>>;

/// Payload 注册表。
///
/// 实现响应解码的动态分发，替代 C++ 的大 `switch`。
///
/// # 设计要点
///
/// - 注册表是全局单例（`OnceLock`），运行时不可变。
/// - 协议变更时只需注册新的解码器，无需修改现有代码。
/// - 对应 C++ 中 `Build()` 的 `switch` 语句的反向操作（解码）。
#[derive(Debug)]
pub struct PayloadRegistry {
    /// 方法 ID → 解码器函数 的映射
    decoders: HashMap<SomeIPMethod, PayloadDecoder>,
}

impl PayloadRegistry {
    /// 创建新的注册表。
    pub fn new() -> Self {
        PayloadRegistry {
            decoders: HashMap::new(),
        }
    }

    /// 注册解码器。
    ///
    /// # 参数
    ///
    /// * `method` - 方法 ID
    /// * `decoder` - 解码器函数
    pub fn register(&mut self, method: SomeIPMethod, decoder: PayloadDecoder) {
        self.decoders.insert(method, decoder);
    }

    /// 根据方法 ID 解码 payload。
    ///
    /// # 参数
    ///
    /// * `method` - 方法 ID
    /// * `data` - 字节序列
    ///
    /// # 错误
    ///
    /// 当方法 ID 未注册时返回 `UnknownMethod`。
    pub fn decode(&self, method: SomeIPMethod, data: &[u8]) -> SomeIPResult<Box<dyn Payload>> {
        match self.decoders.get(&method) {
            Some(decoder) => decoder(data),
            None => Err(SomeIPError::UnknownMethod(method as u16)),
        }
    }
}

impl Default for PayloadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化默认注册表（注册所有已实现的 payload 解码器）。
///
/// 对应 C++ 中 `Build()` 的 `switch` 语句覆盖的所有方法。
///
/// # 使用示例
///
/// ```rust
/// # use crate::someip::payload::init_default_registry;
/// let registry = init_default_registry();
/// ```
pub fn init_default_registry() -> PayloadRegistry {
    let mut registry = PayloadRegistry::new();

    // 注册 MediaPayload 解码器
    registry.register(SomeIPMethod::SetMedia, |data| {
        Ok(Box::new(MediaPayload::decode(data)?))
    });
    registry.register(SomeIPMethod::GetMedia, |data| {
        Ok(Box::new(MediaPayload::decode(data)?))
    });

    // 注册 SubscribePayload 解码器
    registry.register(SomeIPMethod::Subscribe, |data| {
        Ok(Box::new(SubscribePayload::decode(data)?))
    });

    // 注册 SetNetworkPayload 解码器
    registry.register(SomeIPMethod::SetNetwork, |data| {
        Ok(Box::new(SetNetworkPayload::decode(data)?))
    });
    registry.register(SomeIPMethod::GetNetwork, |data| {
        Ok(Box::new(SetNetworkPayload::decode(data)?))
    });

    // 注册 CameraROI Payload 解码器
    registry.register(SomeIPMethod::SetROI, |data| {
        Ok(Box::new(SetCameraROIPayload::decode(data)?))
    });
    registry.register(SomeIPMethod::GetROI, |data| {
        Ok(Box::new(GetCameraROIPayload::decode(data)?))
    });

    // 注册 AlgROIPayload 解码器
    registry.register(SomeIPMethod::SetAlgROI, |data| {
        Ok(Box::new(AlgROIPayload::decode(data)?))
    });
    registry.register(SomeIPMethod::GetAlgROI, |data| {
        Ok(Box::new(AlgROIPayload::decode(data)?))
    });

    // 注册 CamExclusivePayload 解码器
    registry.register(SomeIPMethod::SetCamExclusive, |data| {
        Ok(Box::new(CamExclusivePayload::decode(data)?))
    });

    // 注册 FindOrOfferPayload 解码器
    registry.register(SomeIPMethod::FindOrOffer, |data| {
        Ok(Box::new(FindOrOfferPayload::decode(data)?))
    });

    // 注册 ConfigPayload 解码器
    registry.register(SomeIPMethod::SetConfig, |data| {
        Ok(Box::new(ConfigPayload::decode(data)?))
    });
    registry.register(SomeIPMethod::GetConfig, |data| {
        Ok(Box::new(ConfigPayload::decode(data)?))
    });

    registry
}
