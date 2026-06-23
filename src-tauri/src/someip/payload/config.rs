//! ConfigPayload 实现
//!
//! 对应 C++ `setConfigPayload`（someip_protocol.cpp:111-132）。
//!
//! # 字节布局
//!
//! payload 为 JSON 配置文件的内容（Compact 格式）。
//!
//! C++ 实现：
//! 1. 读取 `config/camera_config.json` 文件
//! 2. 解析为 `QJsonDocument`
//! 3. 返回 `doc.toJson(QJsonDocument::Compact)`（JSON 字节）
//!
//! Rust 实现：
//! - 使用 `serde_json` 读取和解析 JSON
//! - `encode()` 返回 JSON 的 UTF-8 字节
//! - `decode()` 从 JSON 字节解析为 `ConfigPayload`

use crate::someip::error::{SomeIPError, SomeIPResult};
use crate::someip::method::SomeIPMethod;
use crate::someip::payload::{Payload, PayloadCodec};
use serde_json::Value;

/// ConfigPayload（配置）。
///
/// 对应 C++ `setConfigPayload`（someip_protocol.cpp:111-132）。
///
/// payload 为 JSON 格式的配置文件内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPayload {
    /// JSON 配置内容（`serde_json::Value`）
    pub config: Value,
}

impl ConfigPayload {
    /// 创建新的 ConfigPayload。
    pub fn new(config: Value) -> Self {
        ConfigPayload { config }
    }

    /// 从 JSON 文件创建 ConfigPayload。
    ///
    /// 对应 C++ 中读取 `config/camera_config.json` 文件。
    ///
    /// # 参数
    ///
    /// * `file_path` - JSON 文件路径
    ///
    /// # 错误
    ///
    /// 当文件不存在或 JSON 格式错误时返回 `ConfigError`。
    pub fn from_file(file_path: &str) -> SomeIPResult<Self> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| SomeIPError::ConfigError(format!("无法读取文件 {}: {}", file_path, e)))?;

        let config: Value = serde_json::from_str(&content)
            .map_err(|e| SomeIPError::ConfigError(format!("JSON 解析失败: {}", e)))?;

        Ok(ConfigPayload { config })
    }

    /// 从 JSON 字符串创建 ConfigPayload。
    pub fn from_json(json_str: &str) -> SomeIPResult<Self> {
        let config: Value = serde_json::from_str(json_str)
            .map_err(|e| SomeIPError::ConfigError(format!("JSON 解析失败: {}", e)))?;

        Ok(ConfigPayload { config })
    }

    /// 序列化为字节数组（JSON Compact 格式）。
    ///
    /// 对应 C++ `doc.toJson(QJsonDocument::Compact)`。
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.config).unwrap_or_default()
    }

    /// 从字节数组反序列化。
    ///
    /// # 参数
    ///
    /// * `bytes` - JSON 字节序列
    ///
    /// # 错误
    ///
    /// 当 JSON 格式错误时返回 `CodecError`。
    pub fn from_bytes(bytes: &[u8]) -> SomeIPResult<Self> {
        let config: Value = serde_json::from_slice(bytes)
            .map_err(|e| SomeIPError::codec_error(format!("JSON 解析失败: {}", e)))?;

        Ok(ConfigPayload { config })
    }

    /// 返回 JSON 字符串（Compact 格式）。
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_default()
    }
}

impl Payload for ConfigPayload {
    fn method_id(&self) -> SomeIPMethod {
        SomeIPMethod::SetConfig
    }

    fn encode(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl PayloadCodec for ConfigPayload {
    fn decode(data: &[u8]) -> SomeIPResult<Self> {
        Self::from_bytes(data)
    }
}

impl Default for ConfigPayload {
    fn default() -> Self {
        ConfigPayload {
            config: Value::Object(serde_json::Map::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_payload_to_bytes_roundtrip() {
        let json_str = r#"{"camera_id": 1, "enable": true}"#;
        let payload = ConfigPayload::from_json(json_str).unwrap();
        let bytes = payload.to_bytes();

        let parsed = ConfigPayload::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.config, payload.config);
    }
}
