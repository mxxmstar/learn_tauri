# ONVIF 模块使用指南

## 模块概述

本模块实现了 ONVIF 协议的核心功能，基于 `reqwest` + `quick-xml` 手动封装，不依赖复杂的第三方 ONVIF 库，方便后续扩展。

### 已实现功能

1. **设备发现（WS-Discovery）**：通过 UDP 多播发现局域网内的 ONVIF 设备
2. **设备管理（GetDeviceInformation）**：获取设备制造商、型号、固件版本等信息
3. **设备能力查询（GetCapabilities）**：获取设备支持的功能（媒体、PTZ、事件等）

## 模块架构

```
src-tauri/src/onvif/
├── mod.rs          # 模块主入口，定义 OnvifClient 统一门面
├── soap.rs        # SOAP 协议基础（构造/解析 SOAP 信封、WS-Security 认证）
├── device.rs       # 设备管理（GetDeviceInformation）
├── capabilities.rs # 设备能力查询（GetCapabilities）
└── error.rs       # 错误类型定义

src-tauri/src/udp/
└── discovery.rs   # WS-Discovery 设备发现（UDP 多播）
```

## 前端调用示例

### 1. 设备发现

```typescript
import { invoke } from '@tauri-apps/api';

// 发现局域网内的 ONVIF 设备（超时 5 秒）
const devices = await invoke('discover_devices', { timeoutMs: 5000 }) as DiscoveredDevice[];

console.log('发现设备数量：', devices.length);
devices.forEach((d: any) => {
  console.log('  设备名称：', d.name || 'Unknown');
  console.log('  服务地址：', d.xaddrs[0]);
  console.log('  型号：', d.hardware || 'Unknown');
});
```

**返回数据类型 `DiscoveredDevice`：**
```typescript
interface DiscoveredDevice {
  uuid: string;           // 设备 UUID
  xaddrs: string[];       // 设备服务地址列表
  types: string[];         // 设备类型
  scopes: string[];        // 设备属性范围
  metadata_version: number; // 元数据版本
}
```

### 2. 获取设备信息

```typescript
// 获取设备基本信息（需要认证）
const deviceInfo = await invoke('get_device_info', {
  deviceUri: 'http://192.168.1.100/onvif/device_service',
  username: 'admin',
  password: '12345',
}) as OnvifDeviceInfo;

console.log('制造商：', deviceInfo.manufacturer);
console.log('型号：', deviceInfo.model);
console.log('固件版本：', deviceInfo.firmware_version);
console.log('序列号：', deviceInfo.serial_number);
```

**返回数据类型 `OnvifDeviceInfo`：**
```typescript
interface OnvifDeviceInfo {
  manufacturer: string;     // 设备制造商
  model: string;            // 设备型号
  firmware_version: string;  // 固件版本
  serial_number: string;    // 设备序列号
  hardware_id: string;      // 硬件 ID
}
```

### 3. 获取设备能力

```typescript
// 获取设备能力（判断设备支持哪些功能）
const caps = await invoke('get_capabilities', {
  deviceUri: 'http://192.168.1.100/onvif/device_service',
  username: 'admin',
  password: '12345',
}) as OnvifCapabilities;

console.log('设备服务地址：', caps.device_xaddr);
console.log('支持媒体配置：', caps.has_media);
console.log('支持 PTZ 控制：', caps.has_ptz);
console.log('支持事件订阅：', caps.has_events);
```

**返回数据类型 `OnvifCapabilities`：**
```typescript
interface OnvifCapabilities {
  device_xaddr: string;  // 设备服务地址
  has_media: boolean;     // 是否支持媒体配置
  has_ptz: boolean;      // 是否支持 PTZ 控制
  has_events: boolean;    // 是否支持事件订阅
  has_imaging: boolean;   // 是否支持成像设置
  has_analytics: boolean; // 是否支持视频分析
  media_xaddr?: string;    // 媒体服务地址（可选）
  ptz_xaddr?: string;     // PTZ 服务地址（可选）
}
```

## Rust 后端调用示例

如果你需要在 Rust 后端代码中使用 ONVIF 模块（而不通过 Tauri 命令），可以直接调用：

```rust
use crate::onvif::{OnvifClient, udp::discovery::discover};

// 1. 设备发现
let devices = discover(5000).await?;
if let Some(device) = devices.first() {
    let device_uri = device.get_first_xaddr().unwrap();
    
    // 2. 连接设备
    let client = OnvifClient::connect(
        device_uri,
        Some("admin"),
        Some("12345"),
    )?;
    
    // 3. 获取设备信息
    let info = client.get_device_info().await?;
    println!("制造商：{}", info.manufacturer);
    
    // 4. 获取设备能力
    let caps = client.get_capabilities().await?;
    println!("支持媒体：{}", caps.has_media);
}
```

## 扩展新功能

模块架构设计方便后续扩展。添加新 ONVIF 操作（如 PTZ 控制、媒体配置）的步骤：

### 示例：添加 PTZ 控制功能

1. **创建新模块文件** `src/onvif/ptz.rs`：

```rust
//! ONVIF PTZ 控制模块
//!
//! 实现 PTZ 相关操作（连续移动、绝对移动、相对移动等）。

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::soap::{build_soap_envelope, send_soap_request};
use serde::Serialize;

/// PTZ 节点信息
#[derive(Debug, Clone, Serialize)]
pub struct PtzNode {
    pub token: String,
    pub name: String,
    // ... 其他字段
}

/// 获取 PTZ 节点列表
pub async fn get_nodes(client: &OnvifClient) -> OnvifResult<Vec<PtzNode>> {
    // 1. 构造 GetNodes SOAP Body
    let body = r#"<GetNodes xmlns="http://www.onvif.org/ver20/ptz/wsdl"/>"#;
    
    // 2. 包装成完整 SOAP 信封
    let envelope = build_soap_envelope(
        body,
        "http://www.onvif.org/ver20/ptz/wsdl/GetNodes",
        client.auth.as_ref(),
    )?;
    
    // 3. 发送请求
    let response = send_soap_request(
        &client.http_client,
        &client.device_uri,
        "http://www.onvif.org/ver20/ptz/wsdl/GetNodes",
        &envelope,
    ).await?;
    
    // 4. 解析响应
    parse_nodes_response(&response)
}
```

2. **在 `mod.rs` 中注册新模块**：

```rust
pub mod soap;
pub mod device;
pub mod capabilities;
pub mod ptz;  // 新增
pub mod error;
```

3. **在 `OnvifClient` 中添加便捷方法**：

```rust
impl OnvifClient {
    // ... 现有方法 ...
    
    /// 获取 PTZ 节点列表
    pub async fn get_ptz_nodes(&self) -> OnvifResult<Vec<PtzNode>> {
        ptz::get_nodes(self).await
    }
}
```

4. **添加 Tauri 命令**（在 `lib.rs` 中）：

```rust
#[tauri::command]
async fn get_ptz_nodes(
    device_uri: String,
    username: Option<String>,
    password: Option<String>,
) -> OnvifResult<Vec<PtzNode>> {
    let client = match OnvifClient::connect(
        &device_uri,
        username.as_deref(),
        password.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => return OnvifResult::err(e),
    };
    
    match client.get_ptz_nodes().await {
        Ok(nodes) => OnvifResult::ok(nodes),
        Err(e) => OnvifResult::err(e),
    }
}
```

## 注意事项

### 1. WS-Security 认证

ONVIF 设备通常要求 WS-Security 用户名令牌认证。本模块已实现 `PasswordDigest` 认证方式：

- 使用 SHA1 计算密码摘要：`PasswordDigest = Base64(SHA1(Nonce + Created + Password))`
- Nonce 使用 UUID v4 生成 16 字节随机数
- 时间戳格式：`2024-06-24T12:34:56.000Z`（RFC 3339）

### 2. 设备发现可靠性

`udp::discovery::discover()` 当前实现是简化版本：

- XML 解析使用字符串匹配（非完整 XML 解析）
- 实际项目中应使用 `quick-xml` 解析 ProbeMatch 响应
- 多播接收超时后返回已发现的设备列表

### 3. SOAP 响应解析

当前 `device.rs` 和 `capabilities.rs` 中的响应解析是占位实现：

- `parse_device_information_response()` 返回固定占位数据
- `get_capabilities()` 未完整解析响应中的能力信息
- 生产环境需要使用 `quick-xml` 解析完整的 SOAP 响应 XML

## 依赖项

本模块使用的第三方库：

- `reqwest`：HTTP 客户端（发送 SOAP 请求）
- `quick-xml`：XML 处理（可选，当前使用字符串构建 SOAP 信封）
- `uuid`：生成随机 Nonce
- `base64`：Base64 编码（密码摘要、Nonce）
- `sha1`：SHA1 哈希（密码摘要计算）
- `serde`：序列化（返回数据到前端）

## 参考资料

- ONVIF 官方规范：https://www.onvif.org/specs/
- WS-Security 规范：https://docs.oasis-open.org/wss/
- WS-Discovery 规范：https://specs.xmlsoap.org/ws/2005/04/discovery/
