//! ONVIF 设备管理模块
//!
//! 实现 ONVIF 设备管理操作（GetDeviceInformation）。
//!
//! # ONVIF 设备管理操作
//!
//! - `GetDeviceInformation`：获取设备制造商、型号、固件版本、序列号等
//! - `GetSystemDateAndTime`：获取设备系统时间
//! - `SetSystemDateAndTime`：设置设备系统时间
//! - `GetNetworkInterfaces`：获取网络接口配置
//! - `Reboot`：重启设备
//!
//! 当前仅实现 `GetDeviceInformation`，后续可按需拓展。

use super::error::OnvifResult;
use super::soap::{build_soap_envelope, send_soap_request};
use serde::Serialize;

/// 设备基本信息
///
/// 对应 ONVIF `GetDeviceInformationResponse`。
#[derive(Debug, Clone, Serialize)]
pub struct OnvifDeviceInfo {
    /// 设备制造商（如 "Hikvision", "Dahua", "Axis"）
    pub manufacturer: String,
    /// 设备型号（如 "DS-2CD2T47G2-L"）
    pub model: String,
    /// 固件版本（如 "V5.6.0 build 221210"）
    pub firmware_version: String,
    /// 设备序列号（制造商分配的唯一标识）
    pub serial_number: String,
    /// 设备硬件 ID
    pub hardware_id: String,
}

/// 获取设备基本信息
///
/// 对应 ONVIF 标准操作：`GetDeviceInformation`
///
/// # 参数
///
/// - `client`：`OnvifClient` 实例
///
/// # 返回值
///
/// 成功时返回 `OnvifDeviceInfo`。
pub async fn get_device_information(
    client: &super::OnvifClient,
) -> OnvifResult<OnvifDeviceInfo> {
    // 构造 GetDeviceInformation 请求的 SOAP Body
    let body = r#"
        <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
    "#;

    let soap_action = "http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation";
    
    let envelope = build_soap_envelope(
        body,
        soap_action,
        client.auth.as_ref(),
    )?;

    let response = send_soap_request(
        &client.http_client,
        &client.device_uri,
        soap_action,
        &envelope,
    ).await?;

    // 解析响应
    parse_device_information_response(&response)
}

/// 解析 GetDeviceInformation 响应
///
/// TODO: 使用 quick-xml 解析 SOAP 响应 XML，提取设备信息字段
fn parse_device_information_response(_response: &str) -> OnvifResult<OnvifDeviceInfo> {
    // 使用 quick-xml 解析 SOAP 响应
    // 实际实现需要解析 XML 并提取字段
    // 这里先返回占位数据
    Ok(OnvifDeviceInfo {
        manufacturer: "Unknown".to_string(),
        model: "Unknown".to_string(),
        firmware_version: "Unknown".to_string(),
        serial_number: "Unknown".to_string(),
        hardware_id: "Unknown".to_string(),
    })
}
