//! ONVIF 设备能力查询模块
//!
//! 实现 ONVIF 设备能力查询（GetCapabilities）。
//!
//! # ONVIF 设备能力分类
//!
//! ONVIF 将设备能力按功能分类，通过 `GetCapabilities` 获取：
//! - `Device`：设备管理（必需）
//! - `Media`：媒体配置（获取 RTSP 流地址等，常用）
//! - `PTZ`：云台控制（球机必需）
//! - `Events`：事件订阅（移动检测、告警等）
//! - `Imaging`：成像设置（曝光、增益等）
//! - `Analytics`：视频分析（人脸识别、入侵检测等）
//!
//! # 使用说明
//!
//! 获取能力后，可根据 `has_media`、`has_ptz` 等字段判断设备支持的功能，
//! 再决定是否调用对应模块的操作（如媒体配置、PTZ 控制等）。

use super::error::OnvifResult;
use super::soap::{build_soap_envelope, send_soap_request};
use serde::Serialize;

/// 设备能力信息
///
/// 对应 ONVIF `GetCapabilitiesResponse`。
#[derive(Debug, Clone, Serialize)]
pub struct OnvifCapabilities {
    /// 设备服务地址
    pub device_xaddr: String,
    /// 媒体服务地址
    pub media_xaddr: String,
    /// PTZ 服务地址
    pub ptz_xaddr: String,
    /// 事件服务地址
    pub events_xaddr: String,
    /// 成像服务地址
    pub imaging_xaddr: String,
    /// 分析服务地址
    pub analytics_xaddr: String,
    /// 设备是否支持 Media 服务
    #[serde(skip)]
    pub has_media: bool,
    /// 设备是否支持 PTZ 服务
    #[serde(skip)]
    pub has_ptz: bool,
    /// 设备是否支持事件服务
    #[serde(skip)]
    pub has_events: bool,
}

impl Default for OnvifCapabilities {
    fn default() -> Self {
        Self {
            device_xaddr: String::new(),
            media_xaddr: String::new(),
            ptz_xaddr: String::new(),
            events_xaddr: String::new(),
            imaging_xaddr: String::new(),
            analytics_xaddr: String::new(),
            has_media: false,
            has_ptz: false,
            has_events: false,
        }
    }
}

/// 获取设备能力
pub async fn get_capabilities(
    client: &super::OnvifClient,
) -> OnvifResult<OnvifCapabilities> {
    let body = r#"
        <tds:GetCapabilities xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
            <tds:Category>All</tds:Category>
        </tds:GetCapabilities>
    "#;

    let soap_action = "http://www.onvif.org/ver10/device/wsdl/GetCapabilities";
    
    let envelope = build_soap_envelope(
        body,
        soap_action,
        client.auth.as_ref(),
    )?;

    let _response = send_soap_request(
        &client.http_client,
        &client.device_uri,
        soap_action,
        &envelope,
    ).await?;

    // TODO: 解析响应 XML，提取设备能力信息
    // 当前返回简化数据
    let mut caps = OnvifCapabilities::default();
    caps.device_xaddr = client.device_uri.clone();
    
    Ok(caps)
}
