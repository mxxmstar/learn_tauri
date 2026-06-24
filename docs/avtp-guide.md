# 标准 AVTP 协议解析模块使用指南

## 目录
1. [模块概述](#1-模块概述)
2. [快速开始](#2-快速开始)
3. [API 参考](#3-api-参考)
4. [协议格式详解](#4-协议格式详解)
5. [与 Stonkam 自定义协议的区别](#5-与-stonkam-自定义协议的区别)
6. [常见问题](#6-常见问题)
7. [参考资料](#7-参考资料)

---

## 1. 模块概述

### 1.1 功能简介

`avtp` 模块提供了 **标准 AVTP 协议（IEEE 1722）** 的完整解析功能。

**核心功能**：
- 以太网帧解析
- AVTP Common Stream Header 解析
- 支持多种 subtype（AAF、MJPEG、H.264）
- 流过滤和序列号跟踪
- 与 `pcap` 模块的集成

### 1.2 适用场景

- 解析标准 AVTP 协议数据包
- 开发兼容 AVTP 的应用程序
- 学习标准 AVTP 协议的设计

### 1.3 模块依赖

- `pcap` — 网卡数据包捕获（可选，用于实时抓包）
- `thiserror` — 错误类型派生
- `std::time` — 时间戳处理
- `std::collections::HashMap` — 流过滤和序列号跟踪

---

## 2. 快速开始

### 2.1 安装依赖

本模块已经包含在项目中，无需额外安装。

**确认 `Cargo.toml` 包含以下依赖**：
```toml
[dependencies]
pcap = "2"           # pcap 库（用于实时抓包）
thiserror = "1"      # 错误派生宏
```

### 2.2 基本使用示例

#### 示例 1：解析离线数据

```rust
use crate::avtp::header::AvtpHeader;
use crate::avtp::error::Result;

fn main() -> Result<()> {
    // 1. 读取包含以太网帧的数据文件
    let ethernet_frame = std::fs::read("avtp_capture.bin")?;
    
    // 2. 解析 AVTP Common Stream Header
    let header = AvtpHeader::from_ethernet_frame(&ethernet_frame)?;
    
    // 3. 打印解析结果
    println!("子类型: {:?}", header.subtype);
    println!("版本: {}", header.version);
    println!("序列号: {}", header.sequence_num);
    println!("流 ID: {:016X}", header.stream_id);
    println!("时间戳: {}", header.timestamp);
    println!("包计数: {}", header.packet_count);
    
    Ok(())
}
```

#### 示例 2：与 pcap 模块集成（实时抓包）

```rust
use crate::pcap::capture::Capture;
use crate::avtp::parser::AvtpParser;
use crate::avtp::header::AvtpSubtype;
use std::thread;

fn main() -> Result<()> {
    // 1. 枚举网卡
    let devices = Capture::list_devices()?;
    println!("可用网卡:");
    for (i, dev) in devices.iter().enumerate() {
        println!("  [{}] {} ({})", i, dev.name, dev.description);
    }
    
    // 2. 打开网卡（使用第一个网卡）
    let device_name = &devices[0].name;
    let mut capture = Capture::open(device_name, true, 65536, 1000)?;
    
    // 3. 创建解析器
    let mut parser = AvtpParser::new();
    
    // 4. 添加流过滤器（可选，只接收指定流 ID 的数据包）
    parser.add_stream_filter(0x123456789ABC0001, "Camera1".to_string());
    
    // 5. 启动捕获（通道模式）
    let (capture, rx) = capture.start_with_channel()?;
    
    // 6. 在独立线程中接收并解析数据包
    thread::spawn(move || {
        for pkt in rx {
            // 解析数据包
            match parser.parse_packet(&pkt.data, pkt.timestamp) {
                Ok(Some(avtp_header)) => {
                    println!("接收到 AVTP 数据包:");
                    println!("  子类型: {:?}", avtp_header.subtype);
                    println!("  流 ID: {:016X}", avtp_header.stream_id);
                    println!("  序列号: {}", avtp_header.sequence_num);
                    
                    // 根据 subtype 处理数据
                    match avtp_header.subtype {
                        AvtpSubtype::Mjpeg => {
                            // 解析 MJPEG 数据
                            // ...
                        }
                        AvtpSubtype::H264 => {
                            // 解析 H.264 数据
                            // ...
                        }
                        _ => {}
                    }
                }
                Ok(None) => {
                    // 不是 AVTP 协议，跳过
                }
                Err(e) => {
                    eprintln!("解析错误: {}", e);
                }
            }
        }
    });
    
    // 7. 等待用户中断
    println!("按 Ctrl+C 停止...");
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

---

## 3. API 参考

### 3.1 错误类型（`error.rs`）

```rust
pub enum AvtpError {
    BufferTooShort { need: usize, got: usize },
    InvalidEtherType(u16),
    InvalidSubtype(u8),
    VersionMismatch { expected: u8, actual: u8 },
    StreamIdMismatch { expected: u64, actual: u64 },
    SequenceError { expected: u8, actual: u8 },
    TimestampError(String),
    PayloadLengthError { expected: u16, actual: usize },
    ParseError(String),
    PcapError(String),
}

pub type Result<T> = std::result::Result<T, AvtpError>;
```

### 3.2 协议头部（`header.rs`）

#### `AvtpSubtype` 枚举

```rust
pub enum AvtpSubtype {
    Aaf = 0x00,      // AAF Audio Format
    Mjpeg = 0x07,    // MJPEG Video Format
    H264 = 0x05,    // H.264 Video Format
    Unknown(u8),    // 未知子类型
}
```

#### `AvtpHeader` 结构体

```rust
pub struct AvtpHeader {
    pub subtype: AvtpSubtype,      // 子类型
    pub version: u8,              // 协议版本（3 bits）
    pub sequence_num: u8,         // 序列号（4 bits）
    pub stream_id: u64,           // 流 ID（8 字节）
    pub timestamp: u32,           // 时间戳（4 字节）
    pub gateway_info: u16,        // 网关信息（2 字节）
    pub packet_count: u16,        // 包计数（2 字节）
    pub reserved: [u8; 6],      // 保留字段（6 字节）
}
```

**主要方法**：
- `from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self>` — 从以太网帧解析头部
- `avtp_data_offset() -> usize` — 获取 AVTP 数据起始偏移（固定为 38）
- `get_avtp_data(ethernet_frame: &[u8]) -> Result<&[u8]>` — 获取 AVTP 数据
- `timestamp_to_system_time(timestamp: u32) -> SystemTime` — 将时间戳转换为 SystemTime

#### `MjpegAvtpPacket` 结构体

```rust
pub struct MjpegAvtpPacket {
    pub header: AvtpHeader,          // AVTP Common Stream Header
    pub mjpeg_payload: Vec<u8>,   // MJPEG 负载数据
}
```

**主要方法**：
- `from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self>` — 从以太网帧解析 MJPEG AVTP 数据包

### 3.3 解析器（`parser.rs`）

#### `AvtpParser` 结构体

```rust
pub struct AvtpParser {
    stream_filters: HashMap<u64, String>,   // 流过滤器
    sequence_tracking: HashMap<u64, u8>,   // 序列号跟踪
    packet_count: AtomicU32,                 // 数据包计数
    error_count: AtomicU32,                 // 错误计数
    dropped_packets: AtomicU32,            // 丢包计数
}
```

**主要方法**：
- `new() -> Self` — 创建新的解析器实例
- `add_stream_filter(stream_id: u64, stream_name: String)` — 添加流过滤器
- `remove_stream_filter(stream_id: u64)` — 移除流过滤器
- `clear_stream_filters()` — 清除所有流过滤器
- `parse_packet(ethernet_frame: &[u8], timestamp: SystemTime) -> Result<Option<AvtpHeader>>` — 解析单个数据包
- `parse_mjpeg_packet(ethernet_frame: &[u8]) -> Result<MjpegAvtpPacket>` — 解析 MJPEG AVTP 数据包
- `reset()` — 重置解析器状态
- `get_stats() -> (u32, u32, u32)` — 获取统计信息

### 3.4 辅助函数（`parser.rs`）

```rust
/// 解析以太网帧中的 EtherType
pub fn parse_ethertype(ethernet_frame: &[u8]) -> Option<u16>

/// 检查是否为 Stonkam 自定义协议
pub fn is_stonkam_protocol(ethernet_frame: &[u8]) -> bool

/// 检查是否为标准 AVTP 协议
pub fn is_standard_avtp(ethernet_frame: &[u8]) -> bool
```

---

## 4. 协议格式详解

### 4.1 以太网帧格式

```
偏移量    长度    字段名
0         6       目标 MAC 地址
6         6       源 MAC 地址
12        2       EtherType（协议类型）
14        N       协议数据（Payload）
```

**EtherType 值**：
- `0x22F0` — 标准 AVTP 协议（本模块使用）
- `0x0800` — IPv4
- `0x0806` — ARP
- `0x86DD` — IPv6

### 4.2 AVTP Common Stream Header 格式

**总长度**：24 字节（以太网帧偏移 14-37）

| 以太网帧偏移量 | 头部内偏移量 | 长度（字节） | 字段名 | 说明 |
|---|---|---|---|---|
| 14 | 0 | 1 | **subtype** | 子类型（例如：0x07 = MJPEG） |
| 15 | 1 | 1 | **version_seq** | version (3 bits) + sequence_num (4 bits) + reserved (1 bit) |
| 16-23 | 2-9 | 8 | **stream_id** | 流 ID（唯一标识一个 AVTP 流） |
| 24-27 | 10-13 | 4 | **timestamp** | 时间戳（纳秒为单位） |
| 28-29 | 14-15 | 2 | **gateway_info** | 网关信息（保留字段） |
| 30-31 | 16-17 | 2 | **packet_count** | 包计数（在流中的序号） |
| 32-37 | 18-23 | 6 | **reserved** | 保留字段 |

### 4.3 subtype 详解

**subtype 字段**（以太网帧偏移 14）用于标识 AVTP 数据的格式。

| subtype 值 | 格式名称 | 说明 |
|---|---|---|
| `0x00` | AAF Audio | 纯音频格式，用于传输未压缩的音频数据 |
| `0x05` | H.264 Video | H.264 视频格式，用于传输 H.264 编码的视频数据 |
| `0x07` | MJPEG Video | Motion JPEG 视频格式，用于传输 JPEG 图像序列 |
| 其他 | 保留/私有 | 未分配或厂商私有 |

### 4.4 序列号详解

**序列号字段**（以太网帧偏移 15，bit 1-4）用于检测丢包。

- 序列号是 4 bits，取值范围 0-15，模 16 循环
- 接收端可以通过检查序列号是否连续来判断是否丢包
- 如果 `sequence_num != (last_sequence_num + 1) % 16`，表示丢包

### 4.5 时间戳详解

**时间戳字段**（以太网帧偏移 24-27）表示数据采集时间。

- 单位是纳秒（nanoseconds）
- 从 1970-01-01 00:00:00 UTC 开始
- 用于音视频同步

---

## 5. 与 Stonkam 自定义协议的区别

### 5.1 对比表格

| 对比项 | 标准 AVTP (IEEE 1722) | Stonkam 自定义协议 |
|---|---|---|
| **EtherType** | `0x22F0` | `0x0022` |
| **协议头部长度** | 24 字节（Common Stream Header） | 24 字节（自定义格式） |
| **流 ID 字段** | 有（8 字节） | 无 |
| **时间戳字段** | 有（4 字节） | 无 |
| **序列号字段** | 有（4 bits） | 无 |
| **帧标志位** | 无 | 有（起始/结束标志） |
| **嵌入式参数** | 无 | 有（JPEG 数据前 12 字节） |
| **协议复杂度** | 高（支持多种格式、流管理） | 低（仅支持 JPEG） |
| **标准化程度** | IEEE 标准 | 厂商私有 |

### 5.2 选择建议

**使用标准 AVTP 协议**：
- 需要与其他厂商的设备互操作
- 需要支持多种音视频格式
- 需要时间戳同步、流管理等功能

**使用 Stonkam 自定义协议**：
- 设备是 Stonkam 制造的
- 只需要解析 JPEG 视频流
- 对协议复杂度要求低

---

## 6. 常见问题

### 6.1 编译错误：`cannot find type 'SystemTime' in this scope`

**原因**：未导入 `SystemTime`。

**解决方法**：在文件顶部添加：
```rust
use std::time::SystemTime;
```

### 6.2 解析错误：`无效的 EtherType: 期望 0x22F0，实际 0x0800`

**原因**：数据包不是 AVTP 协议（可能是 IPv4 数据包）。

**解决方法**：在解析前检查 EtherType：
```rust
let ethertype = u16::from_be_bytes([pkt.data[12], pkt.data[13]]);
if ethertype == 0x22F0 {
    // 解析 AVTP 协议
}
```

### 6.3 序列号错误：`序列号错误: 期望 5，实际 7`

**原因**：检测到丢包或乱序。

**解决方法**：
1. 检查网络连接是否稳定
2. 增加 pcap 缓冲区大小
3. 使用 `get_stats()` 查看丢包统计

### 6.4 时间戳错误：`时间戳错误: 时钟不同步`

**原因**：发送端和接收端的时钟不同步。

**解决方法**：
1. 使用 NTP 同步时钟
2. 忽略时间戳错误（如果不需要音视频同步）
3. 使用相对时间戳（而不是绝对时间戳）

### 6.5 性能优化建议

1. **使用通道模式**：比回调模式更高效
2. **调整缓冲区大小**：根据网络带宽调整 `PACKET_ARRAY_SIZE`
3. **多线程处理**：将解析和显示放在不同线程
4. **流过滤**：只接收需要的流，减少解析开销

---

## 7. 参考资料

### 7.1 协议规范

- **IEEE 1722-2016**：AVTP 标准文档
- **IEEE 1722.1-2013**：AVDECC（AVTP 设备管理）
- **AVnu Alliance**：https://avnu.org/

### 7.2 开发工具

- **Wireshark**：网络协议分析器（支持 AVTP 解析）
- **tcpdump**：命令行数据包分析器
- **pcap 文档**：https://www.tcpdump.org/manpages/pcap.3pcap.html
- **Linux AVTP 工具**：https://github.com/AVnu/avtp

### 7.3 相关标准

- **IEEE 802.3**：以太网标准
- **IEEE 1588**：PTP（精确时间协议，用于时间戳同步）
- **RFC 4553**：RTP（实时传输协议，类似 AVTP）

---

**文档版本**：v1.0  
**编写时间**：2026-06-24  
**作者**：AI Assistant（基于标准 AVTP 协议分析）
