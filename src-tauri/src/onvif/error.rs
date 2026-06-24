//! ONVIF 模块错误类型
//!
//! 定义 ONVIF 协议操作中可能发生的所有错误类型。

use thiserror::Error;

/// ONVIF 操作中可能发生的错误
#[derive(Error, Debug)]
pub enum OnvifError {
    /// HTTP 请求失败
    #[error("HTTP 请求失败 [{method}] {url}: {source}")]
    HttpRequest {
        method: String,
        url: String,
        #[source]
        source: reqwest::Error,
    },

    /// HTTP 响应状态码异常
    #[error("HTTP 响应错误 [{method}] {url}: 状态码 {status}")]
    HttpResponse {
        method: String,
        url: String,
        status: u16,
        body: String,
    },

    /// XML 解析失败
    #[error("XML 解析失败: {0}")]
    XmlParse(#[from] quick_xml::Error),

    /// ONVIF 设备返回 SOAP Fault
    #[error("ONVIF 设备错误 [{fault_code}]: {fault_string}")]
    SoapFault {
        fault_code: String,
        fault_string: String,
        detail: Option<String>,
    },

    /// 设备发现超时
    #[error("设备发现超时（等待 {timeout_ms} 毫秒后未收到响应）")]
    DiscoveryTimeout { timeout_ms: u64 },

    /// 设备不支持该功能
    #[error("设备不支持该功能: {0}")]
    UnsupportedFeature(String),

    /// WS-Security 认证失败
    #[error("认证失败: {0}")]
    Authentication(String),

    /// 无效的参数
    #[error("无效参数: {0}")]
    InvalidArgument(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 其他错误（字符串形式）
    #[error("ONVIF 错误: {0}")]
    Other(String),
}

/// OnvifResult 类型别名
pub type OnvifResult<T> = Result<T, OnvifError>;
