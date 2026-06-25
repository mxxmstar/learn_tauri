# 串口通信模块使用说明

## 模块概述

本模块提供跨平台的串口通信能力，支持：
- 基本的串口操作（打开/关闭/配置/读写）
- 数据帧解析（支持自定义协议）
- 协议扩展接口（通过 `ProtocolParser` trait）
- **同步和异步两种使用模式**

设计风格与项目中的 `telnet` 模块保持一致。

## 同步和异步模式

本模块同时提供同步和异步两种使用模式，用户可以根据需要选择合适的方式：

### 同步模式
- 方法名以 `_sync` 结尾（如 `open_sync()`, `write_sync()`）
- 直接调用，阻塞当前线程
- 适用于简单的命令行工具、同步 API 等场景
- 不需要 async/await 运行时

### 异步模式
- 方法名不带后缀（如 `open()`, `write()`）
- 内部使用 `tokio::task::spawn_blocking` 包装同步操作
- 不会阻塞 tokio 运行时，适合在 async 上下文中使用
- 需要 tokio 运行时

### 如何选择？

| 场景 | 推荐模式 | 原因 |
|------|----------|------|
| 命令行工具、简单脚本 | 同步模式 | 不需要 async 运行时，代码更简单 |
| Web 服务器、Tauri 后端 | 异步模式 | 不阻塞 tokio 运行时，支持高并发 |
| GUI 应用程序 | 异步模式 | 不阻塞 UI 线程 |
| 性能敏感的实时系统 | 同步模式 | 避免 spawn_blocking 的额外开销 |

## 模块结构

```
serial/
├── mod.rs          # 模块入口，重新导出常用类型
├── client.rs       # 核心客户端实现（SerialClient），支持同步和异步
├── error.rs        # 错误类型定义（SerialError）
├── config.rs       # 配置管理（SerialConfig）
├── types.rs        # 数据类型定义
├── protocol.rs     # 协议解析 trait 和内置解析器
└── README.md      # 本文档
```

## 快速开始

### 1. 列出可用串口

**同步模式：**
```rust
use crate::serial::SerialClient;

let ports = SerialClient::list_ports().unwrap();
for port in ports {
    println!("可用串口: {}", port);
}
```

**异步模式：**
```rust
use crate::serial::SerialClient;

// list_ports() 是同步方法，但通常很快完成
let ports = SerialClient::list_ports().unwrap();
for port in ports {
    println!("可用串口: {}", port);
}
```

前端调用示例（TypeScript）：
```typescript
import { invoke } from '@tauri-apps/api';

const ports = await invoke('serial_list_ports') as string[];
console.log('可用串口:', ports);
```

### 2. 打开串口

**同步模式：**
```rust
use crate::serial::{SerialClient, SerialConfig};
use crate::serial::types::*;

let config = SerialConfig::new("COM1")
    .baud_rate(115200)
    .data_bits(DataBits::Eight)
    .stop_bits(StopBits::One)
    .parity(Parity::None);

let client = SerialClient::new(config)?;
client.open_sync()?;  // 注意：同步方法以 _sync 结尾
```

**异步模式：**
```rust
use crate::serial::{SerialClient, SerialConfig};
use crate::serial::types::*;

let config = SerialConfig::new("COM1")
    .baud_rate(115200)
    .data_bits(DataBits::Eight)
    .stop_bits(StopBits::One)
    .parity(Parity::None);

let client = SerialClient::new(config)?;
client.open().await?;  // 注意：异步方法不带后缀
```

前端调用示例（TypeScript）：
```typescript
const result = await invoke('serial_open', {
    config: {
        portName: 'COM1',
        baudRate: 115200,
        dataBits: 8,
        stopBits: 1,
        parity: 'None',
        flowControl: 'None',
        timeoutMs: 1000,
    }
});
```

### 3. 写入数据

**同步模式：**
```rust
let data = b"hello".to_vec();
let bytes_written = client.write_sync(&data)?;  // 同步方法
println!("写入 {} 字节", bytes_written);
```

**异步模式：**
```rust
let data = b"hello".to_vec();
let bytes_written = client.write(&data).await?;  // 异步方法
println!("写入 {} 字节", bytes_written);
```

前端调用示例（TypeScript）：
```typescript
const data = new TextEncoder().encode('hello');
const result = await invoke('serial_write', {
    data: Array.from(data)
});
```

### 4. 读取数据

**同步模式：**
```rust
let max_bytes = 1024;
let data = client.read_sync(max_bytes)?;  // 同步方法
println!("收到数据: {:?}", data);
```

**异步模式：**
```rust
let max_bytes = 1024;
let data = client.read(max_bytes).await?;  // 异步方法
println!("收到数据: {:?}", data);
```

前端调用示例（TypeScript）：
```typescript
const data = await invoke('serial_read', { maxBytes: 1024 }) as number[];
const text = new TextDecoder().decode(new Uint8Array(data));
console.log('收到数据:', text);
```

### 5. 关闭串口

**同步模式：**
```rust
client.close_sync()?;  // 同步方法
```

**异步模式：**
```rust
client.close().await?;  // 异步方法
```

前端调用示例（TypeScript）：
```typescript
await invoke('serial_close');
```

## 配置说明

### SerialConfig 配置项

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `port_name` | `String` | 平台相关 | 串口名称（Windows: "COM1", Linux: "/dev/ttyUSB0"） |
| `baud_rate` | `u32` | 115200 | 波特率（常见值：9600, 19200, 38400, 57600, 115200） |
| `data_bits` | `DataBits` | `DataBits::Eight` | 数据位（5, 6, 7, 8） |
| `stop_bits` | `StopBits` | `StopBits::One` | 停止位（1, 2） |
| `parity` | `Parity` | `Parity::None` | 校验位（None, Odd, Even） |
| `flow_control` | `FlowControl` | `FlowControl::None` | 流控制（None, Software, Hardware） |
| `timeout_ms` | `u64` | 1000 | 读取超时（毫秒） |
| `read_buffer_size` | `usize` | 4096 | 读取缓冲区大小（字节） |

### 配置验证

`SerialConfig::validate()` 方法会验证配置的有效性：
- 端口名称不能为空
- 超时时间必须大于 0
- 缓冲区大小必须至少为 64 字节

## 协议解析

### 内置解析器

#### 1. DelimiterParser（分隔符解析器）

使用指定的分隔符来识别数据帧边界。

```rust
use crate::serial::protocol::DelimiterParser;

// 使用换行符作为分隔符（类似 Telnet 文本协议）
let parser = DelimiterParser::new(b"\n");

// 使用自定义分隔符
let parser = DelimiterParser::new(&[0xAA, 0xBB]);

// 设置是否包含分隔符
let parser = DelimiterParser::new(b"\n")
    .include_delimiter(true);

// 设置最大帧长度
let parser = DelimiterParser::new(b"\n")
    .max_frame_length(64 * 1024);
```

#### 2. LengthPrefixParser（长度前缀解析器）

使用固定长度的前缀字段来指示后续数据的长度。

```rust
use crate::serial::protocol::LengthPrefixParser;

// 使用 2 字节大端序长度前缀
let parser = LengthPrefixParser::new(2, true).unwrap();

// 使用 4 字节小端序长度前缀
let parser = LengthPrefixParser::new(4, false).unwrap();

// 设置长度字段是否包含自身长度
let parser = LengthPrefixParser::new(2, true)
    .unwrap()
    .length_includes_self(true);

// 设置最大帧长度
let parser = LengthPrefixParser::new(2, true)
    .unwrap()
    .max_frame_length(64 * 1024 * 1024);
```

### 自定义协议

实现 `ProtocolParser` trait 来支持自定义协议：

```rust
use crate::serial::protocol::{ProtocolParser, ParseResult};

/// 自定义协议解析器
struct MyProtocolParser;

impl ProtocolParser for MyProtocolParser {
    fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
        // 自定义解析逻辑
        // 返回 ParseResult::Complete(data, consumed) 表示解析成功
        // 返回 ParseResult::Incomplete 表示数据不完整
        // 返回 ParseResult::Error(msg) 表示解析错误
        ParseResult::Incomplete
    }

    fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
        // 自定义编码逻辑
        // 返回编码后的完整帧数据
        data.to_vec()
    }
}
```

### 使用协议解析器

**同步模式：**
```rust
use std::sync::Arc;
use crate::serial::protocol::MyProtocolParser;

// 创建自定义协议解析器
let parser = Arc::new(MyProtocolParser);

// 设置到客户端（同步方法）
client.set_protocol_parser_sync(parser);

// 清除协议解析器（同步方法）
client.clear_protocol_parser_sync();
```

**异步模式：**
```rust
use std::sync::Arc;
use crate::serial::protocol::MyProtocolParser;

// 创建自定义协议解析器
let parser = Arc::new(MyProtocolParser);

// 设置到客户端（异步方法）
client.set_protocol_parser(parser).await;

// 清除协议解析器（异步方法）
client.clear_protocol_parser().await;
```

## 错误处理

模块使用 `SerialError` 枚举来封装所有可能的错误：

| 错误类型 | 说明 |
|----------|------|
| `OpenError` | 端口打开失败（权限不足、端口不存在、端口被占用等） |
| `PortClosed` | 端口已关闭 |
| `NotConnected` | 未连接（尝试在未打开串口时执行操作） |
| `ConfigError` | 配置错误（参数无效等） |
| `IoError` | IO 错误（读取/写入失败等） |
| `Timeout` | 超时错误（操作超时时返回此错误） |
| `ProtocolError` | 协议解析错误（帧格式错误、校验失败等） |
| `Internal` | 内部错误（其他未分类的错误） |

## Tauri 命令

前端可以通过 Tauri 命令来调用串口功能：

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `serial_list_ports` | 无 | `Vec<String>` | 列出所有可用的串口 |
| `serial_open` | `config: SerialConfig` | `()` | 打开串口 |
| `serial_close` | 无 | `()` | 关闭串口 |
| `serial_write` | `data: Vec<u8>` | `usize` | 写入数据，返回写入的字节数 |
| `serial_read` | `maxBytes: usize` | `Vec<u8>` | 读取数据，返回读取的字节数组 |
| `serial_get_status` | 无 | `ConnectionStatus` | 获取连接状态 |

## 注意事项

1. **跨平台兼容性**：
   - Windows: 端口名称格式为 "COM1", "COM2", ...
   - Linux: 端口名称格式为 "/dev/ttyUSB0", "/dev/ttyACM0", ...
   - macOS: 端口名称格式为 "/dev/tty.usbserial", "/dev/tty.usbmodem", ...

2. **权限问题**：
   - Linux/macOS: 用户需要加入 `dialout` 组（Linux）或拥有串口设备的读写权限

3. **超时设置**：
   - 读取操作会受到 `timeout_ms` 配置的影响
   - 如果超时时间内未收到完整数据帧，会返回 `SerialError::Timeout`

4. **缓冲区管理**：
   - 读取缓冲区大小由 `read_buffer_size` 配置控制
   - 使用协议解析器时，内部会循环读取直到解析出完整帧

5. **同步和异步模式混用**：
   - 可以在同一个 `SerialClient` 实例上混用同步和异步方法
   - 但是，不建议在多个线程/任务中同时调用读写方法（无论是同步还是异步）
   - 如果需要在多个线程/任务中共享客户端，请使用克隆（`SerialClient` 实现了 `Clone`）

## 完整示例

### 同步模式完整示例

```rust
use crate::serial::{SerialClient, SerialConfig};
use crate::serial::types::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 列出可用串口
    let ports = SerialClient::list_ports()?;
    println!("可用串口: {:?}", ports);

    // 2. 创建配置
    let config = SerialConfig::new("COM1")
        .baud_rate(115200)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .timeout_ms(1000);

    // 3. 创建客户端
    let client = SerialClient::new(config)?;

    // 4. 打开串口
    client.open_sync()?;
    println!("串口已打开");

    // 5. 写入数据
    let data = b"hello".to_vec();
    let n = client.write_sync(&data)?;
    println!("写入 {} 字节", n);

    // 6. 读取数据
    let received = client.read_sync(1024)?;
    println!("收到数据: {:?}", received);

    // 7. 关闭串口
    client.close_sync()?;
    println!("串口已关闭");

    Ok(())
}
```

### 异步模式完整示例

```rust
use crate::serial::{SerialClient, SerialConfig};
use crate::serial::types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 列出可用串口
    let ports = SerialClient::list_ports()?;
    println!("可用串口: {:?}", ports);

    // 2. 创建配置
    let config = SerialConfig::new("COM1")
        .baud_rate(115200)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .timeout_ms(1000);

    // 3. 创建客户端
    let client = SerialClient::new(config)?;

    // 4. 打开串口
    client.open().await?;
    println!("串口已打开");

    // 5. 写入数据
    let data = b"hello".to_vec();
    let n = client.write(&data).await?;
    println!("写入 {} 字节", n);

    // 6. 读取数据
    let received = client.read(1024).await?;
    println!("收到数据: {:?}", received);

    // 7. 关闭串口
    client.close().await?;
    println!("串口已关闭");

    Ok(())
}
```

## 常见问题

### Q: 如何判断串口是否已打开？

A: 使用同步方法 `client.is_connected_sync()` 或异步方法 `client.is_connected().await`。

### Q: 如何设置自定义波特率？

A: 在创建 `SerialConfig` 时，使用 `.baud_rate()` 方法设置。注意：某些设备可能不支持非标准波特率。

### Q: 为什么读取操作会阻塞？

A: 
- 同步模式：读取操作会阻塞当前线程，直到收到数据或超时。
- 异步模式：读取操作在后台线程中执行，不会阻塞 tokio 运行时。

### Q: 如何同时处理多个串口？

A: 创建多个 `SerialClient` 实例，每个实例管理一个串口连接。`SerialClient` 实现了 `Clone`，可以方便地共享配置。

### Q: 同步和异步方法可以混用吗？

A: 可以。但是，不建议在多个线程/任务中同时调用读写方法。如果需要在多个线程/任务中共享客户端，请使用克隆。

## 作者与维护

- 实现者：[Your Name]
- 维护者：[Your Name]
- 版本：1.1.0  # 更新版本号，添加同步/异步模式支持
- 更新日期：2026-06-25
