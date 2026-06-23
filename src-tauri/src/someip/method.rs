//! SomeIP 方法 ID 枚举（稳定接口契约）
//!
//! 对应 C++ `enum class SomeIPMethod`（someip_protocol.h:55-91）。
//! 这是协议稳定接口的根基，方法 ID 定义不可变更。
//! 新协议版本应新增变体，而非修改现有变体的值。

#![allow(non_camel_case_types)]

/// SomeIP 方法 ID 枚举。
///
/// `#[repr(u16)]` 保证内存布局与 C++ `enum class SomeIPMethod` 一致，
/// 可直接转换为 `u16` 写入 SomeIP 报文。
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SomeIPMethod {
    /// 未知方法（默认值，对应 C++ `unKnown = 0x0000`）
    Unknown = 0x0000,

    /// 获取数据表
    GetDataSheet = 0x0001,
    /// 设置摄像头独占模式
    SetCamExclusive = 0x0011,
    /// 释放摄像头独占模式
    EraseCamExclusive = 0x0019,

    /// 设置 ROI 区域
    SetROI = 0x0101,
    /// 获取 ROI 区域
    GetROI = 0x0103,

    /// 订阅事件
    Subscribe = 0x0131,
    /// 取消订阅事件
    UnSubscribe = 0x0132,

    /// 订阅报警事件
    SubscribeAlarm = 0x0141,
    /// 取消订阅报警事件
    UnSubscribeAlarm = 0x0142,

    /// 获取算法 ROI 区域
    GetAlgROI = 0x0143,
    /// 设置算法 ROI 区域
    SetAlgROI = 0x0144,

    /// 获取网络配置
    GetNetwork = 0x0191,
    /// 设置网络配置
    SetNetwork = 0x0192,

    /// 获取媒体配置
    GetMedia = 0x0171,
    /// 设置媒体配置
    SetMedia = 0x0172,

    /// 获取算法配置
    GetAlg = 0x0181,
    /// 设置算法配置
    SetAlg = 0x0182,

    /// 获取系统配置
    GetSystem = 0x0201,

    /// 恢复出厂设置
    ResetFactory = 0x0211,

    /// 获取配置
    GetConfig = 0x0150,
    /// 设置配置
    SetConfig = 0x0151,

    /// 服务发现或提供服务
    FindOrOffer = 0x8100,
}

impl SomeIPMethod {
    /// 返回所有方法 ID 变体（用于遍历或注册表初始化）。
    pub const ALL: &'static [SomeIPMethod] = &[
        SomeIPMethod::Unknown,
        SomeIPMethod::GetDataSheet,
        SomeIPMethod::SetCamExclusive,
        SomeIPMethod::EraseCamExclusive,
        SomeIPMethod::SetROI,
        SomeIPMethod::GetROI,
        SomeIPMethod::Subscribe,
        SomeIPMethod::UnSubscribe,
        SomeIPMethod::SubscribeAlarm,
        SomeIPMethod::UnSubscribeAlarm,
        SomeIPMethod::GetAlgROI,
        SomeIPMethod::SetAlgROI,
        SomeIPMethod::GetNetwork,
        SomeIPMethod::SetNetwork,
        SomeIPMethod::GetMedia,
        SomeIPMethod::SetMedia,
        SomeIPMethod::GetAlg,
        SomeIPMethod::SetAlg,
        SomeIPMethod::GetSystem,
        SomeIPMethod::ResetFactory,
        SomeIPMethod::GetConfig,
        SomeIPMethod::SetConfig,
        SomeIPMethod::FindOrOffer,
    ];

    /// 判断该方法是否为"Get"类请求（无 payload，length=0）。
    ///
    /// 对应 C++ `Build()` 中 `SetLength(0)` 的分支。
    pub fn is_get_request(&self) -> bool {
        matches!(
            self,
            SomeIPMethod::GetDataSheet
                | SomeIPMethod::GetMedia
                | SomeIPMethod::GetAlgROI
                | SomeIPMethod::GetROI
                | SomeIPMethod::GetNetwork
                | SomeIPMethod::GetAlg
                | SomeIPMethod::GetSystem
                | SomeIPMethod::GetConfig
                | SomeIPMethod::UnSubscribe
        )
    }

    /// 判断该方法是否需要 payload（Set 类请求）。
    pub fn has_payload(&self) -> bool {
        !self.is_get_request() && *self != SomeIPMethod::Unknown
    }
}

impl From<u16> for SomeIPMethod {
    fn from(value: u16) -> Self {
        match value {
            0x0001 => SomeIPMethod::GetDataSheet,
            0x0011 => SomeIPMethod::SetCamExclusive,
            0x0019 => SomeIPMethod::EraseCamExclusive,
            0x0101 => SomeIPMethod::SetROI,
            0x0103 => SomeIPMethod::GetROI,
            0x0131 => SomeIPMethod::Subscribe,
            0x0132 => SomeIPMethod::UnSubscribe,
            0x0141 => SomeIPMethod::SubscribeAlarm,
            0x0142 => SomeIPMethod::UnSubscribeAlarm,
            0x0143 => SomeIPMethod::GetAlgROI,
            0x0144 => SomeIPMethod::SetAlgROI,
            0x0191 => SomeIPMethod::GetNetwork,
            0x0192 => SomeIPMethod::SetNetwork,
            0x0171 => SomeIPMethod::GetMedia,
            0x0172 => SomeIPMethod::SetMedia,
            0x0181 => SomeIPMethod::GetAlg,
            0x0182 => SomeIPMethod::SetAlg,
            0x0201 => SomeIPMethod::GetSystem,
            0x0211 => SomeIPMethod::ResetFactory,
            0x0150 => SomeIPMethod::GetConfig,
            0x0151 => SomeIPMethod::SetConfig,
            0x8100 => SomeIPMethod::FindOrOffer,
            _ => SomeIPMethod::Unknown,
        }
    }
}

impl From<SomeIPMethod> for u16 {
    fn from(method: SomeIPMethod) -> Self {
        method as u16
    }
}

impl Default for SomeIPMethod {
    fn default() -> Self {
        SomeIPMethod::Unknown
    }
}
