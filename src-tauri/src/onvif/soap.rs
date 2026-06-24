//! ONVIF SOAP 协议基础模块
//!
//! 负责构造 ONVIF SOAP 请求信封（Envelope）和解析 SOAP 响应。
//!
//! # ONVIF SOAP 协议概述
//!
//! ONVIF 基于 SOAP 1.2 协议，所有操作都封装在 SOAP Envelope 中：
//!
//! ```xml
//! <SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" ...>
//!   <SOAP-ENV:Header>
//!     <!-- 可选的 WS-Security 认证头 -->
//!   </SOAP-ENV:Header>
//!   <SOAP-ENV:Body>
//!     <!-- ONVIF 操作请求体 -->
//!   </SOAP-ENV:Body>
//! </SOAP-ENV:Envelope>
//! ```
//!
//! # 模块职责
//!
//! - `build_soap_envelope`：构造完整 SOAP Envelope XML 字符串
//! - `send_soap_request`：发送 SOAP 请求到设备并获取响应
//! - `extract_soap_body`：从 SOAP 响应中提取 Body 内容
//!
//! # 拓展说明
//!
//! 后续添加新 ONVIF 操作（如 `GetProfiles`、`ContinuousMove`）时，
//! 只需：
//! 1. 在对应模块（如 `device.rs`、`ptz.rs`）中构造操作特有的 XML Body
//! 2. 调用本模块的 `build_soap_envelope` 包装成完整 SOAP 请求
//! 3. 调用 `send_soap_request` 发送请求
//! 4. 解析响应中的 ONVIF 操作结果

use base64::Engine;
use crate::onvif::error::{OnvifError, OnvifResult};

/// SOAP 1.2 命名空间
const SOAP_NS: &str = "http://www.w3.org/2003/05/soap-envelope";

/// 构造完整的 SOAP Envelope XML 字符串
///
/// 使用简单的字符串构建方式，避免 quick_xml Writer API 版本冲突问题。
///
/// # 参数
///
/// - `body`: SOAP Body 内的 XML 内容（ONVIF 操作请求）
/// - `soap_action`: SOAPAction HTTP 头（ONVIF 操作标识，当前未使用但保留接口）
/// - `auth`: 可选的 WS-Security 认证头
///
/// # 返回值
///
/// 返回完整的 SOAP Envelope XML 字符串，可直接作为 HTTP POST 的请求体。
pub fn build_soap_envelope(
    body: &str,
    _soap_action: &str,
    auth: Option<&OnvifAuth>,
) -> OnvifResult<String> {
    // 构造 XML 声明
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    // 构造 SOAP Envelope 开始标签
    xml.push_str("<SOAP-ENV:Envelope xmlns:SOAP-ENV=\"");
    xml.push_str(SOAP_NS);
    xml.push_str("\">\n");

    // 构造 SOAP Header
    xml.push_str("  <SOAP-ENV:Header>\n");
    if let Some(auth) = auth {
        xml.push_str(&build_auth_header(auth));
    }
    xml.push_str("  </SOAP-ENV:Header>\n");

    // 构造 SOAP Body
    xml.push_str("  <SOAP-ENV:Body>\n");
    xml.push_str(body);
    xml.push_str("\n  </SOAP-ENV:Body>\n");

    // 关闭 SOAP Envelope
    xml.push_str("</SOAP-ENV:Envelope>\n");

    Ok(xml)
}

/// WS-Security 认证头
///
/// 存储计算好的 WS-Security 认证字段，
/// 由 `write_auth_header` 函数写入 SOAP Header。
pub struct OnvifAuth {
    /// 用户名
    pub username: String,
    /// 密码摘要（PasswordDigest，Base64 编码）
    pub password_digest: String,
    /// 随机数（Nonce，Base64 编码）
    pub nonce: String,
    /// 创建时间戳（ISO 8601 格式）
    pub created: String,
}

impl OnvifAuth {
    /// 根据用户名密码创建 WS-Security 认证头
    pub fn new(username: &str, password: &str) -> Self {
        // 使用 UUID v4 生成 16 字节随机 Nonce
        let nonce_uuid = uuid::Uuid::new_v4();
        let nonce_bytes = nonce_uuid.as_bytes();
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce_bytes);

        // 获取当前 UTC 时间戳（ISO 8601 格式）
        let now = std::time::SystemTime::now();
        let created = format_rfc3339(now);

        // 计算 PasswordDigest = Base64(SHA1(Nonce + Created + Password))
        // 注意：Nonce 使用原始字节（不是 Base64 解码），Created 使用字符串字节
        let created_bytes = created.as_bytes();
        let password_bytes = password.as_bytes();

        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(nonce_bytes);      // 使用原始 16 字节
        hasher.update(created_bytes);
        hasher.update(password_bytes);
        let digest = hasher.finalize();
        let digest_b64 = base64::engine::general_purpose::STANDARD.encode(digest);

        Self {
            username: username.to_string(),
            password_digest: digest_b64,
            nonce: nonce_b64,
            created,
        }
    }
}

/// 构造 WS-Security 认证头的 XML 字符串
///
/// 返回格式化后的 XML 字符串，包含：
/// - `<wsse:Security>` 标签
/// - `<wsse:UsernameToken>` 标签
/// - 用户名、密码摘要、Nonce、时间戳等元素
fn build_auth_header(auth: &OnvifAuth) -> String {
    const WSSE_NS: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
    const WSU_NS: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";
    const PASSWORD_DIGEST_TYPE: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";

    let mut xml = String::new();

    // <wsse:Security>
    xml.push_str("    <wsse:Security xmlns:wsse=\"");
    xml.push_str(WSSE_NS);
    xml.push_str("\" xmlns:wsu=\"");
    xml.push_str(WSU_NS);
    xml.push_str("\">\n");

    // <wsse:UsernameToken>
    xml.push_str("      <wsse:UsernameToken>\n");

    // <wsse:Username>
    xml.push_str("        <wsse:Username>");
    xml.push_str(&auth.username);
    xml.push_str("</wsse:Username>\n");

    // <wsse:Password>
    xml.push_str("        <wsse:Password Type=\"");
    xml.push_str(PASSWORD_DIGEST_TYPE);
    xml.push_str("\">");
    xml.push_str(&auth.password_digest);
    xml.push_str("</wsse:Password>\n");

    // <wsse:Nonce>
    xml.push_str("        <wsse:Nonce>");
    xml.push_str(&auth.nonce);
    xml.push_str("</wsse:Nonce>\n");

    // <wsu:Created>
    xml.push_str("        <wsu:Created>");
    xml.push_str(&auth.created);
    xml.push_str("</wsu:Created>\n");

    // </wsse:UsernameToken>
    xml.push_str("      </wsse:UsernameToken>\n");

    // </wsse:Security>
    xml.push_str("    </wsse:Security>\n");

    xml
}

/// 通过 reqwest 发送 SOAP 请求
pub async fn send_soap_request(
    http_client: &reqwest::Client,
    url: &str,
    soap_action: &str,
    soap_envelope: &str,
) -> OnvifResult<String> {
    let response = http_client
        .post(url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .header("SOAPAction", soap_action)
        .body(soap_envelope.to_string())
        .send()
        .await
        .map_err(|e| OnvifError::HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            source: e,
        })?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| OnvifError::HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            source: e,
        })?;

    if !status.is_success() {
        return Err(OnvifError::HttpResponse {
            method: "POST".to_string(),
            url: url.to_string(),
            status: status.as_u16(),
            body,
        });
    }

    Ok(body)
}

/// 将 SystemTime 格式化为 RFC 3339 / ISO 8601 格式
/// 
/// ONVIF WS-Security 要求时间戳格式为：2024-06-24T12:34:56.000Z
fn format_rfc3339(time: std::time::SystemTime) -> String {
    // 使用简单的手动格式化（避免引入 time/chrono 依赖）
    // 格式：YYYY-MM-DDTHH:MM:SS.sssZ
    #[cfg(target_family = "unix")]
    {
        // Unix 系统可以使用 libc 获取分解时间
        let duration = time.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();
        
        // 简化：使用 UTC 时间（实际项目应使用 chrono 或 time crate）
        let (year, mon, day, hour, min, sec) = seconds_to_utc(secs);
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                year, mon, day, hour, min, sec, millis)
    }
    
    #[cfg(not(target_family = "unix"))]
    {
        // Windows 或其他平台：使用简单格式
        let duration = time.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();
        
        let (year, mon, day, hour, min, sec) = seconds_to_utc(secs);
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                year, mon, day, hour, min, sec, millis)
    }
}

/// 将 UNIX 时间戳（秒）转换为 UTC 日期时间
/// 
/// 简化实现，仅用于示例（实际应使用专业时间库）
fn seconds_to_utc(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    // UNIX 纪元：1970-01-01 00:00:00 UTC
    // 一年 = 365 * 86400 = 31536000 秒
    // 闰年额外 = 86400 秒
    
    let mut remaining = secs;
    let mut year: i64 = 1970;
    
    loop {
        let days_in_year = if is_leap_year(year as i32) { 366 } else { 365 };
        let year_secs: u64 = days_in_year * 86400;
        if remaining < year_secs {
            break;
        }
        remaining -= year_secs;
        year += 1;
    }
    
    // 计算月份和日期
    let month_days = if is_leap_year(year as i32) {
        [31u32, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    let mut month: u32 = 1;
    let mut day: u32 = 1;
    let mut rem = remaining;
    
    for &days in &month_days {
        let month_secs: u64 = days as u64 * 86400;
        if rem < month_secs {
            day = ((rem / 86400) as u32) + 1;
            rem %= 86400;
            break;
        }
        rem -= month_secs;
        month += 1;
    }
    
    let hour = (rem / 3600) as u32;
    rem %= 3600;
    let minute = (rem / 60) as u32;
    let second = (rem % 60) as u32;
    
    (year as i32, month, day, hour, minute, second)
}

/// 判断是否为闰年
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
