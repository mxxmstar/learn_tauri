# Stonkam 自定义 AVTP 协议解析模块使用指南

## 目录
1. [模块概述](#1-模块概述)
2. [快速开始](#2-快速开始)
3. [API 参考](#3-api-参考)
4. [协议格式详解](#4-协议格式详解)
5. [与标准 AVTP 的区别](#5-与标准-avtp-的区别)
6. [常见问题](#6-常见问题)
7. [参考资料](#7-参考资料)

---

## 1. 模块概述

### 1.1 功能简介

`stonkam_avtp` 模块提供了 **Stonkam 车载视频监控设备**使用的自定义协议（EtherType 0x0022）的完整解析功能。

**核心功能**：
- 以太网帧解析
- 自定义协议头部解析
- JPEG 帧重组（通过帧起始/结束标志）
- JPEG 嵌入式头部解析（图像参数提取）
- 与 `pcap` 模块的集成

### 1.2 适用场景

- 解析 Stonkam 设备发送的 video 流
- 开发基于 Stonkam 设备的应用程序
- 学习自定义以太网协议的设计

### 1.3 模块依赖

- `pcap` — 网卡数据包捕获（可选，用于实时抓包）
- `thiserror` — 错误类型派生
- `std::time` — 时间戳处理

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
use crate::stonkam_avtp::header::StonkamAvtpHeader;
use crate::stonkam_avtp::error::Result;

fn main() -> Result<()> {
    // 1. 读取包含以太网帧的数据文件
    let ethernet_frame = std::fs::read("stonkam_capture.bin")?;
    
    // 2. 解析协议头部
    let header = StonkamAvtpHeader::from_ethernet_frame(&ethernet_frame)?;
    
    // 3. 打印解析结果
    println!("帧起始标志: {}", header.frame_start);
    println!("帧结束标志: {}", header.frame_end);
    println!("负载长度: {} 字节", header.payload_len);
    
    // 4. 解析 JPEG 嵌入式头部
    let jpeg_data = StonkamAvtpHeader::get_jpeg_data(&ethernet_frame)?;
    let embedded_header = StonkamAvtpHeader::parse_jpeg_embedded_header(jpeg_data)?;
    
    println!("图像尺寸: {}x{}", embedded_header.width, embedded_header.height);
    println!("JPEG 质量因子: {}", embedded_header.qp);
    
    Ok(())
}
```

#### 示例 2：与 pcap 模块集成（实时抓包）

```rust
use crate::pcap::capture::Capture;
use crate::stonkam_avtp::parser::StonkamAvtpParser;
use crate::stonkam_avtp::header::StonkamAvtpHeader;
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
    let mut parser = StonkamAvtpParser::new();
    
    // 4. 启动捕获（通道模式）
    let (capture, rx) = capture.start_with_channel()?;
    
    // 5. 在独立线程中接收并解析数据包
    thread::spawn(move || {
        for pkt in rx {
            // 检查 EtherType
            if pkt.data.len() >= 14 {
                let ethertype = u16::from_be_bytes([pkt.data[12], pkt.data[13]]);
                if ethertype == 0x0022 {
                    // 解析数据包
                    match parser.parse_packet(&pkt.data, pkt.timestamp) {
                        Ok(Some(jpeg_data)) => {
                            println!("接收到完整的 JPEG 帧: {} 字节", jpeg_data.len());
                            // 解码并显示 JPEG 图像
                            // ...
                        }
                        Ok(None) => {
                            // 中间包，继续等待
                        }
                        Err(e) => {
                            eprintln!("解析错误: {}", e);
                        }
                    }
                }
            }
        }
    });
    
    // 6. 等待用户中断
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
pub enum StonkamAvtpError {
    BufferTooShort { need: usize, got: usize },
    InvalidEtherType(u16),
    InvalidFrameFlags { start: u8, end: u8 },
    JpegDataTooShort(usize),
    JpegDecodeError(String),
    PacketOrderError { expected: String, actual: String },
    InvalidImageParameter { param: String, value: u32 },
    PcapError(String),
}

pub type Result<T> = std::result::Result<T, StonkamAvtpError>;
```

### 3.2 协议头部（`header.rs`）

#### `StonkamAvtpHeader` 结构体

```rust
pub struct StonkamAvtpHeader {
    pub raw_ethernet_frame: Vec<u8>,  // 以太网帧原始数据
    pub frame_start: bool,              // 帧起始标志
    pub frame_end: bool,                // 帧结束标志
    pub payload_len: u16,              // 负载长度
    pub reserved: [u8; 20],          // 保留字段
}
```

**主要方法**：
- `from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self>` — 从以太网帧解析头部
- `jpeg_data_offset() -> usize` — 获取 JPEG 数据起始偏移量（固定为 38）
- `get_jpeg_data(ethernet_frame: &[u8]) -> Result<&[u8]>` — 获取 JPEG 数据
- `parse_jpeg_embedded_header(jpeg_data: &[u8]) -> Result<JpegEmbeddedHeader>` — 解析嵌入式头部

#### `JpegEmbeddedHeader` 结构体

```rust
pub struct JpegEmbeddedHeader {
    pub qp: u8,        // JPEG 质量因子（1-100）
    pub width: u16,     // 图像宽度（像素）
    pub height: u16,    // 图像高度（像素）
    pub rst_int: u16,   // JPEG 重启标记间隔
    pub rst_count: u16, // 帧计数器（低 10 位有效）
    pub raw: Vec<u8>,   // 原始 12 字节数据
}
```

### 3.3 解析器（`parser.rs`）

#### `StonkamAvtpParser` 结构体

```rust
pub struct StonkamAvtpParser {
    jpeg_buffer: Vec<u8>,         // JPEG 数据缓冲区
    receiving_frame: bool,        // 是否正在接收一帧
    frame_count: AtomicU32,      // 已接收的完整帧计数
    packet_count: AtomicU32,     // 已接收的数据包计数
    dropped_frames: AtomicU32,   // 丢帧计数
    max_frame_size: usize,       // 最大 JPEG 帧大小
}
```

**主要方法**：
- `new() -> Self` — 创建新的解析器实例
- `with_config(max_frame_size: usize) -> Self` — 创建可配置的解析器实例
- `parse_packet(ethernet_frame: &[u8], timestamp: SystemTime) -> Result<Option<Vec<u8>>>` — 解析单个数据包
- `get_stats() -> (u32, u32, u32)` — 获取统计信息

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
- `0x0022` — Stonkam 自定义协议（本模块使用）
- 注意：原 C++ 代码只检查 `packet[12] == 0x22`（低字节），未检查高字节

### 4.2 自定义协议头部格式

**总长度**：24 字节（以太网帧偏移 14-37）

| 以太网帧偏移量 | 协议内偏移量 | 长度（字节） | 字段名 | 说明 |
|---|---|---|---|---|
| 14 | 0 | 1 | 保留/版本 | 未知用途 |
| 15 | 1 | 1 | **帧起始标志** | bit 0 = 1 表示起始包 |
| 16-19 | 2-5 | 4 | 保留字段 | 未知用途 |
| 20-33 | 6-19 | 14 | 保留字段 | 未知用途 |
| 34 | 20 | 2 | **负载长度** | JPEG 数据长度（大端） |
| 36 | 22 | 1 | **帧结束标志** | bit 4 = 1 表示结束包 |
| 37 | 23 | 1 | 保留字段 | 未知用途 |
| 38+ | 24+ | N | **JPEG 数据** | 原始 JPEG 字节流 |

### 4.3 JPEG 嵌入式头部格式

**位置**：JPEG 数据的前 12 字节（以太网帧偏移 38-49）

| 偏移量（相对于 JPEG 数据起始） | 长度（字节） | 字段名 | 说明 |
|---|---|---|---|
| 0 | 5 | 未知 | 未知用途 |
| 5 | 1 | **质量因子 (QP)** | JPEG 质量参数（1-100） |
| 6 | 1 | **图像宽度** | 实际宽度 = 值 × 8 像素 |
| 7 | 1 | **图像高度** | 实际高度 = 值 × 8 像素 |
| 8-9 | 2 | **重启间隔 (RST)** | JPEG 重启标记间隔 |
| 10-11 | 2 | **帧计数** | 某种计数器（低 10 位有效） |
| 12+ | N | **JPEG 数据** | 实际的 JPEG 字节流 |

### 4.4 帧标志位详解

**帧起始标志**（以太网帧偏移 15）：
```
Bit 7 6 5 4 3 2 1 0
    - - - - - - - - -
    ^^^^^^^^^^^^^^^^ 保留（未知）
                    ^  帧起始标志（1 = 起始包，0 = 中间包）
```

**帧结束标志**（以太网帧偏移 36）：
```
Bit 7 6 5 4 3 2 1 0
    - - - - - - - - -
    ^^^^^^^^^^^^^^^^ 保留（未知）
          ^          帧结束标志（1 = 结束包，0 = 中间包）
```

**包类型判断逻辑**：
| 帧起始标志 | 帧结束标志 | 包类型 |
|---|---|---|
| 1 | 0 | 起始包（JPEG 帧的第一部分） |
| 0 | 0 | 中间包（JPEG 帧的中间部分） |
| 0 | 1 | 结束包（JPEG 帧的最后一部分） |
| 1 | 1 | 完整包（JPEG 帧在一个包内完整传输） |

---

## 5. 与标准 AVTP 的区别

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

### 5.2 选择建议

**使用 Stonkam 自定义协议**：
- 设备是 Stonkam 制造的
- 只需要解析 JPEG 视频流
- 对协议复杂度要求低

**使用标准 AVTP 协议**：
- 需要与其他厂商的设备互操作
- 需要支持多种音视频格式
- 需要时间戳同步、流管理等功能

---

## 6. 常见问题

### 6.1 编译错误：`cannot find type 'SystemTime' in this scope`

**原因**：未导入 `SystemTime`。

**解决方法**：在文件顶部添加：
```rust
use std::time::SystemTime;
```

### 6.2 解析错误：`无效的 EtherType: 0x0800`

**原因**：数据包不是 Stonkam 协议（可能是 IPv4 数据包）。

**解决方法**：在解析前检查 EtherType：
```rust
let ethertype = u16::from_be_bytes([pkt.data[12], pkt.data[13]]);
if ethertype == 0x0022 {
    // 解析 Stonkam 协议
}
```

### 6.3 JPEG 解码失败

**原因**：
1. JPEG 数据损坏
2. 嵌入式头部参数错误
3. 丢包导致帧不完整

**解决方法**：
1. 检查 `dropped_frames` 统计信息
2. 增加 `max_frame_size` 配置
3. 使用 Wireshark 抓包分析

### 6.4 性能优化建议

1. **使用通道模式**：比回调模式更高效
2. **调整缓冲区大小**：根据网络带宽调整 `PACKET_ARRAY_SIZE`
3. **多线程处理**：将解析和显示放在不同线程

---

## 7. 参考资料

### 7.1 协议规范

- **Stonkam 官网**：https://www.stonkam.com/
- **标准 AVTP (IEEE 1722)**：https://standards.ieee.org/ieee/1722/6157/

### 7.2 开发工具

- **Wireshark**：网络协议分析器
- **tcpdump**：命令行数据包分析器
- **pcap 文档**：https://www.tcpdump.org/manpages/pcap.3pcap.html

### 7.3 JPEG 标准

- **ITU-T T.81**：JPEG 标准文档
- **JFIF 规范**：https://www.w3.org/Graphics/JPEG/jfif.txt

---

**文档版本**：v1.0  
**编写时间**：2026-06-24  
**作者**：AI Assistant（基于 Stonkam 协议分析）
