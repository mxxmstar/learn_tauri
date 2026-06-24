//! ONVIF 模块
//!
//! 基于 `reqwest` + `quick-xml` 封装的 ONVIF 客户端模块，
//! 提供设备发现、设备管理、设备能力查询等功能。
//!
//! # 模块结构
//!
//! ```text
//! onvif/
//! ├── mod.rs          # 模块主入口，定义 OnvifClient 统一门面
//! ├── soap.rs        # SOAP 协议基础（构造/解析 SOAP 信封）
//! ├── device.rs       # 设备管理（GetDeviceInformation 等）
//! ├── capabilities.rs # 设备能力查询（GetCapabilities）
//! └── error.rs       # 错误类型
//! ```
//!
//! # 快速开始
//!
//! ```ignore
//! use crate::onvif::OnvifClient;
//! use crate::udp::discovery::discover;
//!
//! // 设备发现
//! let devices = discover(5000).await?;
//!
//! // 连接设备并获取设备信息
//! let client = OnvifClient::connect(
//!     &devices[0],
//!     Some(("admin", "12345")),
//! )?;
//! let info = client.get_device_info().await?;
//! let caps = client.get_capabilities().await?;
//! ```

pub mod soap;
pub mod device;
pub mod capabilities;
pub mod error;

// 重新导出核心类型，方便外部使用
pub use soap::OnvifAuth;
pub use device::OnvifDeviceInfo;
pub use capabilities::OnvifCapabilities;
pub use error::{OnvifError, OnvifResult};

/// ONVIF 客户端统一门面
///
/// 封装 SOAP 客户端，提供简洁的异步 API。
pub struct OnvifClient {
    /// 设备服务地址
    device_uri: String,
    /// 认证信息（若有）
    auth: Option<OnvifAuth>,
    /// HTTP 客户端
    http_client: reqwest::Client,
}

impl OnvifClient {
    /// 连接到指定设备
    ///
    /// # 参数
    ///
    /// - `device_uri`：设备服务地址（从 `DiscoveredDevice.xaddrs` 获取，
    ///   或手动指定如 `"http://192.168.1.100/onvif/device_service"`）
    /// - `username`：用户名（可选）
    /// - `password`：密码（可选）
    pub fn connect(
        device_uri: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> OnvifResult<Self> {
        let auth = match (username, password) {
            (Some(u), Some(p)) => Some(OnvifAuth::new(u, p)),
            _ => None,
        };

        let http_client = reqwest::Client::new();

        Ok(Self {
            device_uri: device_uri.to_string(),
            auth,
            http_client,
        })
    }

    /// 获取设备 URI
    pub fn device_uri(&self) -> &str {
        &self.device_uri
    }

    /// 获取设备基本信息
    pub async fn get_device_info(&self) -> OnvifResult<OnvifDeviceInfo> {
        device::get_device_information(self).await
    }

    /// 获取设备能力
    pub async fn get_capabilities(&self) -> OnvifResult<OnvifCapabilities> {
        capabilities::get_capabilities(self).await
    }
}
