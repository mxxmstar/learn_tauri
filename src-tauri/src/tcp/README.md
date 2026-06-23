# TCP 模块使用说明

本模块基于 `tokio::net` 和 `tokio_util::codec` 实现完整的异步 TCP 通信功能，包括服务端和客户端，支持消息帧化（解决粘包/半包）、全双工通信、心跳保活。

## 目录

- [模块结构](#模块结构)
- [核心特性](#核心特性)
- [TCP vs UDP](#tcp-vs-udp)
- [消息帧化原理](#消息帧化原理)
- [快速开始](#快速开始)
  - [服务端示例](#服务端示例)
  - [客户端示例](#客户端示例)
  - [回声服务器示例](#回声服务器示例)
- [配置说明](#配置说明)
  - [TcpServerConfig](#tcpserverconfig)
  - [TcpClientConfig](#tcpclientconfig)
- [消息协议](#消息协议)
- [API 参考](#api-参考)
  - [TcpServer](#tcpserver)
  - [TcpClient](#tcpclient)
  - [TcpService](#tcpservice)
- [典型应用场景](#典型应用场景)
  - [场景一：请求-响应](#场景一请求-响应)
  - [场景二：服务端推送](#场景二服务端推送)
  - [场景三：多客户端聊天室](#场景三多客户端聊天室)
  - [场景四：心跳保活](#场景四心跳保活)
  - [场景五：自动重连](#场景五自动重连)
- [注意事项](#注意事项)
- [扩展建议](#扩展建议)

---

## 模块结构

```
tcp/
├── mod.rs      # 模块主入口，定义 TcpService 统一管理接口
├── config.rs   # 配置管理（服务端/客户端配置、心跳、重连）
├── message.rs  # 消息协议定义与编解码（JSON 结构化消息）
├── codec.rs    # 消息帧化（LengthDelimitedCodec，解决粘包/半包）
├── server.rs   # TCP 异步服务端（多连接并发）
├── client.rs   # TCP 异步客户端（读写分离、自动重连）
└── README.md   # 使用说明文档（本文件）
```

### 各文件职责

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块入口，重新导出类型，提供 `TcpService` 统一管理接口 |
| `config.rs` | 定义 `TcpServerConfig`、`TcpClientConfig`，包含心跳、重连、超时等配置项 |
| `message.rs` | 定义 `MessageType` 和 `TcpMessage`，实现 JSON 编解码 |
| `codec.rs` | 封装 `tokio_util::codec::LengthDelimitedCodec`，配置长度前缀帧化 |
| `server.rs` | 实现 `TcpServer`，多连接并发、回调机制、广播、心跳保活 |
| `client.rs` | 实现 `TcpClient`，读写分离、全双工、Ping/Pong、自动重连 |

---

## 核心特性

- **消息帧化**：使用 `tokio_util::codec::LengthDelimitedCodec`，4 字节长度前缀，彻底解决 TCP 粘包/半包问题
- **异步 IO**：基于 tokio 异步运行时，高并发低开销
- **多连接并发**：服务端为每个连接创建独立 tokio 任务，互不阻塞
- **全双工通信**：客户端读写分离（`split()`），可同时发送和接收
- **心跳保活**：内置 Ping/Pong 机制，支持 RTT 测量，防止连接假死
- **自动重连**：客户端支持可配置的自动重连机制（重连次数、间隔）
- **超时控制**：支持连接超时、读写超时
- **优雅停机**：通过 `tokio::sync::watch` 通道实现服务端/客户端的优雅停止
- **广播转发**：服务端支持向所有活跃连接广播消息
- **配置验证**：所有配置在创建时自动验证，避免运行时错误

---

## TCP vs UDP

本项目同时实现了 TCP 和 UDP 模块，下表对比两者差异，帮助选择合适的协议：

| 特性 | TCP (`tcp/`) | UDP (`udp/`) |
|------|-------------|-------------|
| 连接方式 | 面向连接（三次握手） | 无连接 |
| 可靠性 | 可靠传输（保证到达、顺序） | 不可靠（可能丢包、乱序） |
| 消息边界 | 流式（无边界，需帧化） | 数据报（有边界） |
| 拥塞控制 | 有 | 无 |
| 传输效率 | 较低（协议开销） | 高 |
| 适用场景 | 文件传输、命令控制、聊天 | 实时音视频、DNS、心跳 |

**选择建议：**
- 需要可靠传输 → TCP
- 需要低延迟、可容忍丢包 → UDP
- 需要双向持续通信 → TCP（全双工）

---

## 消息帧化原理

TCP 是字节流协议，没有消息边界。本模块使用**长度前缀**方案解决粘包/半包：

```
字节流: [len=10][消息A 10字节][len=5][消息B 5字节]
              ↑ 第1帧 ↑              ↑ 第2帧 ↑
```

### 帧格式

```
┌─────────────────┬──────────────────────────┐
│  长度头 (4 字节) │   消息体 (最多 1 MB)      │
│  大端 u32        │   (JSON 编码的 TcpMessage) │
└─────────────────┴──────────────────────────┘
```

- **长度头**：4 字节大端序无符号整数，表示消息体字节数
- **消息体**：JSON 编码的 `TcpMessage`，最大 1 MB
- **编解码**：由 `tokio_util::codec::LengthDelimitedCodec` 自动处理

### 工作流程

1. **发送端**：`TcpMessage` → JSON 序列化 → codec 添加长度头 → TCP 流
2. **接收端**：TCP 流 → codec 读取长度头并等待完整帧 → 输出 `BytesMut` → JSON 反序列化 → `TcpMessage`

codec 内部自动处理：
- **半包**：帧不完整时缓冲，等待剩余数据
- **粘包**：多个帧粘连时正确切分

---

## 快速开始

### 服务端示例

```rust
use learn_tauri_lib::tcp::{TcpService, TcpServerConfig, TcpMessage};

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. 创建服务端配置
    let config = TcpServerConfig::new("0.0.0.0:8080")
        .with_buffer_size(16384)
        .with_max_connections(2048);

    // 2. 创建服务
    let mut service = TcpService::new_server(config)?;

    // 3. 设置消息回调
    service.on_message(|msg, addr, conn_id| {
        println!("[服务端] 连接 {} ({}) 消息: {}", conn_id, addr, msg.payload);
        Box::pin(async {})
    }).await?;

    // 4. 设置连接事件回调（可选）
    service.on_connection_event(|event| {
        Box::pin(async move {
            match event {
                learn_tauri_lib::tcp::ConnectionEvent::Connected(id, addr) => {
                    println!("[服务端] 新连接: ID={}, 地址={}", id, addr);
                }
                learn_tauri_lib::tcp::ConnectionEvent::Disconnected(id, addr) => {
                    println!("[服务端] 连接断开: ID={}, 地址={}", id, addr);
                }
            }
        })
    }).await?;

    // 5. 启动服务
    service.start_server().await?;
    println!("TCP 服务端已启动，监听 0.0.0.0:8080");

    // 6. 保持运行
    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;

    // 7. 停止服务
    service.stop_server().await
}
```

### 客户端示例

```rust
use learn_tauri_lib::tcp::{TcpService, TcpClientConfig, TcpMessage};

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. 创建客户端配置
    let config = TcpClientConfig::new("127.0.0.1:8080")
        .with_timeout(5000)              // 读写超时 5 秒
        .with_connect_timeout(3000);     // 连接超时 3 秒

    // 2. 创建服务
    let service = TcpService::new_client(config)?;

    // 3. 连接服务端
    service.connect().await?;

    // 4. 发送消息
    service.send(&TcpMessage::data("Hello, Server!")).await?;
    println!("[客户端] 已发送消息");

    // 5. 接收回复
    let msg = service.recv().await?;
    println!("[客户端] 收到回复: {}", msg.payload);

    // 6. Ping 测试（测量往返延迟）
    let rtt = service.ping().await?;
    println!("[客户端] RTT: {}ms", rtt);

    Ok(())
}
```

### 回声服务器示例

回声服务器收到任何消息后原样返回，常用于测试网络连通性：

```rust
use learn_tauri_lib::tcp::server::run_echo_server;

#[tokio::main]
async fn main() -> Result<(), String> {
    run_echo_server("0.0.0.0:8080").await
}
```

---

## 配置说明

### TcpServerConfig

服务端配置。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `listen_addr` | `String` | `"0.0.0.0:8080"` | 监听地址（`IP:Port` 格式） |
| `buffer_size` | `usize` | `8192` | 接收缓冲区大小（字节） |
| `keepalive` | `bool` | `true` | 是否启用 TCP Keep-Alive（系统级保活） |
| `heartbeat` | `bool` | `true` | 是否启用应用层心跳（Ping/Pong） |
| `heartbeat_interval_ms` | `u64` | `30000` | 心跳间隔（毫秒） |
| `max_connections` | `usize` | `1024` | 最大并发连接数 |
| `connection_timeout_ms` | `u64` | `10000` | 单连接读写超时（毫秒，0 表示不超时） |

**构建方法（链式调用）：**

```rust
let config = TcpServerConfig::new("0.0.0.0:8080")
    .with_buffer_size(16384)              // 设置缓冲区大小
    .with_keepalive(true)                 // 启用 TCP 保活
    .with_heartbeat(true)                 // 启用应用层心跳
    .with_heartbeat_interval(15000)       // 心跳间隔 15 秒
    .with_max_connections(2048)           // 最大 2048 连接
    .with_connection_timeout(30000);      // 连接超时 30 秒
```

### TcpClientConfig

客户端配置。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `server_addr` | `String` | `"127.0.0.1:8080"` | 目标服务端地址 |
| `buffer_size` | `usize` | `8192` | 接收缓冲区大小（字节） |
| `connect_timeout_ms` | `u64` | `5000` | 连接超时时间（毫秒） |
| `timeout_ms` | `u64` | `10000` | 读写超时时间（毫秒，0 表示不超时） |
| `keepalive` | `bool` | `true` | 是否启用 TCP Keep-Alive |
| `heartbeat` | `bool` | `true` | 是否启用应用层心跳 |
| `heartbeat_interval_ms` | `u64` | `30000` | 心跳间隔（毫秒） |
| `auto_reconnect` | `bool` | `false` | 是否启用自动重连 |
| `reconnect_interval_ms` | `u64` | `3000` | 重连间隔（毫秒） |
| `max_reconnect_attempts` | `u32` | `0` | 最大重连次数（0 表示无限重试） |

**构建方法（链式调用）：**

```rust
let config = TcpClientConfig::new("127.0.0.1:8080")
    .with_buffer_size(16384)              // 设置缓冲区大小
    .with_connect_timeout(3000)           // 连接超时 3 秒
    .with_timeout(5000)                   // 读写超时 5 秒
    .with_keepalive(true)                 // 启用 TCP 保活
    .with_heartbeat(true)                 // 启用应用层心跳
    .with_heartbeat_interval(20000)       // 心跳间隔 20 秒
    .with_auto_reconnect(true)            // 启用自动重连
    .with_reconnect_interval(2000)        // 重连间隔 2 秒
    .with_max_reconnect_attempts(5);      // 最多重连 5 次
```

---

## 消息协议

本模块使用 JSON 编码的结构化消息，配合长度前缀帧化传输。

### MessageType 消息类型

| 类型 | 说明 | 使用场景 |
|------|------|----------|
| `Data` | 普通数据消息 | 日常通信 |
| `Broadcast` | 广播消息 | 服务端转发给所有连接 |
| `Ping` | 心跳请求 | 连通性检测 |
| `Pong` | 心跳响应 | 回复 Ping（服务端自动回复） |
| `Custom(String)` | 自定义类型 | 业务特定消息 |

### TcpMessage 结构

```json
{
    "msg_type": "Data",
    "payload": "Hello, TCP!",
    "timestamp": 1719000000000
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `msg_type` | `MessageType` | 消息类型 |
| `payload` | `String` | 消息负载（文本内容） |
| `timestamp` | `u64` | Unix 时间戳（毫秒） |

**消息创建方法：**

```rust
// 普通数据消息
let msg = TcpMessage::data("Hello");

// 广播消息
let msg = TcpMessage::broadcast("通知所有人");

// Ping 消息
let ping = TcpMessage::ping();

// Pong 消息（回复 Ping，传入原 Ping 的时间戳）
let pong = TcpMessage::pong(ping.timestamp);

// 自定义类型消息
let msg = TcpMessage::custom("Login", "{\"user\":\"alice\"}");
```

---

## API 参考

### TcpServer

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建服务端实例 |
| `on_message(callback)` | 设置消息回调（接收 `TcpMessage, SocketAddr, ConnectionId`） |
| `on_connection_event(callback)` | 设置连接事件回调（连接建立/断开） |
| `start()` | 启动服务端（非阻塞） |
| `stop()` | 停止服务端 |
| `broadcast(msg)` | 广播消息给所有连接 |
| `get_connections()` | 获取所有活跃连接信息 |
| `connection_count()` | 获取活跃连接数 |
| `local_addr()` | 获取本地绑定地址 |
| `config()` | 获取配置引用 |

### TcpClient

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建客户端实例 |
| `connect()` | 连接服务端 |
| `disconnect()` | 断开连接 |
| `send(msg)` | 发送结构化消息 |
| `send_raw(data)` | 发送原始字节 |
| `recv()` | 接收消息（阻塞，支持超时） |
| `ping()` | Ping 测试，返回 RTT（毫秒） |
| `request(msg)` | 请求-响应模式 |
| `start_recv_loop()` | 启动后台接收循环 |
| `is_connected()` | 是否已连接 |
| `peer_addr()` | 获取服务端地址 |
| `config()` | 获取配置引用 |

### TcpService

统一管理接口，自动判断服务端/客户端模式。

| 方法 | 说明 |
|------|------|
| `new_server(config)` | 创建服务端模式 |
| `new_client(config)` | 创建客户端模式 |
| `on_message(cb)` | 设置消息回调（服务端） |
| `on_connection_event(cb)` | 设置连接事件回调（服务端） |
| `start_server()` | 启动服务端 |
| `stop_server()` | 停止服务端 |
| `broadcast(msg)` | 广播消息（服务端） |
| `get_connections()` | 获取连接列表（服务端） |
| `connection_count()` | 获取连接数（服务端） |
| `connect()` | 客户端连接 |
| `disconnect()` | 客户端断开 |
| `send(msg)` | 客户端发送 |
| `recv()` | 客户端接收 |
| `ping()` | 客户端 Ping |
| `request(msg)` | 客户端请求-响应 |
| `start_recv_loop()` | 客户端后台接收 |
| `is_server()` / `is_client()` | 判断模式 |

---

## 典型应用场景

### 场景一：请求-响应

客户端发送请求，服务端处理后返回响应。

**服务端：**

```rust
use learn_tauri_lib::tcp::{TcpServer, TcpServerConfig, TcpMessage};

let config = TcpServerConfig::new("0.0.0.0:8080");
let mut server = TcpServer::new(config)?;

server.on_message(|msg, addr, conn_id| {
    let payload = msg.payload.clone();
    Box::pin(async move {
        println!("处理来自连接 {} ({}) 的请求: {}", conn_id, addr, payload);
        // 在回调中处理请求并通过共享 writer 回复
    })
}).await;

server.start().await?;
```

**客户端：**

```rust
use learn_tauri_lib::tcp::{TcpClient, TcpClientConfig, TcpMessage};

let config = TcpClientConfig::new("127.0.0.1:8080").with_timeout(5000);
let client = TcpClient::new(config)?;
client.connect().await?;

// 请求-响应模式
let response = client.request(&TcpMessage::data("查询用户列表")).await?;
println!("收到响应: {}", response.payload);
```

### 场景二：服务端推送

服务端主动向客户端推送消息，客户端使用后台接收循环。

```rust
use learn_tauri_lib::tcp::{TcpClient, TcpClientConfig};

let config = TcpClientConfig::new("127.0.0.1:8080");
let client = TcpClient::new(config)?;
client.connect().await?;

// 启动后台接收循环
let mut rx = client.start_recv_loop().await?;

// 在另一个任务中处理收到的推送消息
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        println!("[推送] 收到: {}", msg.payload);
    }
});

// 主线程可以继续发送消息（全双工，发送不受影响）
client.send(&TcpMessage::data("订阅通知")).await?;
```

### 场景三：多客户端聊天室

多个客户端连接服务端，服务端转发消息给所有连接。

**服务端：**

```rust
use learn_tauri_lib::tcp::{TcpServer, TcpServerConfig, TcpMessage};

let config = TcpServerConfig::new("0.0.0.0:8080");
let mut server = TcpServer::new(config)?;
let server_ref = std::sync::Arc::new(tokio::sync::Mutex::new(server));

// 收到消息后广播给所有连接
let server_clone = server_ref.clone();
{
    let mut s = server_ref.lock().await;
    s.on_message(move |msg, addr, conn_id| {
        let server_clone = server_clone.clone();
        let payload = msg.payload.clone();
        Box::pin(async move {
            println!("[聊天室] {} ({}): {}", conn_id, addr, payload);
            // 广播给所有连接
            let broadcast_msg = TcpMessage::broadcast(format!("{}: {}", conn_id, payload));
            let s = server_clone.lock().await;
            let _ = s.broadcast(&broadcast_msg).await;
        })
    }).await;
}

server_ref.lock().await.start().await?;
```

### 场景四：心跳保活

服务端和客户端定期发送 Ping，检测连接是否存活。

**服务端（自动心跳）：**

服务端配置 `heartbeat: true` 后，会自动每隔 `heartbeat_interval_ms` 向每个连接发送 Ping，并自动回复客户端的 Ping。

```rust
let config = TcpServerConfig::new("0.0.0.0:8080")
    .with_heartbeat(true)
    .with_heartbeat_interval(30000);  // 30 秒
```

**客户端 Ping 测试：**

```rust
use std::time::Duration;

// 每 30 秒发送一次 Ping 检测连接
loop {
    match client.ping().await {
        Ok(rtt) => println!("[心跳] 连接正常，RTT: {}ms", rtt),
        Err(e) => {
            println!("[心跳] 连接异常: {}", e);
            break;
        }
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
}
```

### 场景五：自动重连

客户端连接断开后自动重连，适用于需要长期保持连接的场景。

```rust
use learn_tauri_lib::tcp::{TcpClient, TcpClientConfig};

let config = TcpClientConfig::new("127.0.0.1:8080")
    .with_auto_reconnect(true)              // 启用自动重连
    .with_reconnect_interval(3000)          // 每 3 秒重试一次
    .with_max_reconnect_attempts(0);        // 0 表示无限重试

let client = TcpClient::new(config)?;

// 连接失败时会自动重试
client.connect().await?;
```

---

## 注意事项

### 1. TCP 协议特性

- **面向连接**：通信前需建立连接（三次握手），结束后关闭（四次挥手）
- **可靠传输**：保证数据到达、顺序正确，但可能有延迟
- **流式协议**：无消息边界，必须使用帧化（本模块已处理）
- **拥塞控制**：网络拥堵时自动降速

### 2. 消息大小限制

- 单条消息最大 **1 MB**（由 `codec.rs` 的 `MAX_FRAME_LENGTH` 控制）
- 超过限制的消息会被 codec 拒绝并返回错误
- 如需传输更大消息，修改 `MAX_FRAME_LENGTH` 常量

### 3. 心跳机制

- **TCP Keep-Alive**：系统级保活，默认 2 小时无数据才检测（由 OS 控制）
- **应用层心跳**：本模块实现的 Ping/Pong，更灵活可控
- 建议两者都启用，TCP Keep-Alive 作为兜底，应用层心跳用于快速检测

### 4. 读写分离

- 客户端连接后使用 `split()` 分离读写
- **发送**：通过 `Mutex<Option<WriterHalf>>` 保护，支持并发发送
- **接收**：`recv()` 和 `start_recv_loop()` 竞争读取端，**不能同时使用**
- 启动后台接收循环后，读取端所有权转移，`recv()` 和 `ping()` 将不可用，但 `send()` 仍可正常使用

### 5. 超时处理

- `connect_timeout_ms`：仅作用于 `connect()` 方法
- `timeout_ms`：作用于 `send()`、`recv()`、`ping()` 方法
- 超时设置为 `0` 表示永不超时（一直阻塞等待）

### 6. 资源管理

- 服务端停止时会断开所有连接
- 客户端断开会关闭 TCP 连接并停止后台任务
- 建议使用 `tokio::signal::ctrl_c()` 实现优雅停机

### 7. 并发安全

- `TcpClient` 实现了 `Clone`，可在多个异步任务间共享
- `TcpServer` 的回调函数需满足 `Send + Sync + 'static` 约束
- 连接表使用 `Arc<Mutex<HashMap>>` 保护，线程安全

### 8. 在 Tauri 中使用

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    tcp_service: Arc<Mutex<Option<tcp::TcpService>>>,
}

#[tauri::command]
async fn start_tcp_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut service = state.tcp_service.lock().await;
    let mut new_service = tcp::TcpService::new_server(
        tcp::TcpServerConfig::new("0.0.0.0:8080")
    )?;
    new_service.start_server().await?;
    *service = Some(new_service);
    Ok(())
}
```

---

## 扩展建议

### 1. TLS/SSL 加密

使用 `tokio-rustls` 或 `tokio-native-tls` 为 TCP 连接添加加密层，确保通信安全。

### 2. 连接池

实现客户端连接池，复用连接减少握手开销。

### 3. 流式数据传输

对于大文件传输，实现分块流式传输，避免单帧过大。

### 4. 协议升级

支持 Protocol Buffers、MessagePack 等高效二进制编码格式替代 JSON。

### 5. 限流与背压

实现令牌桶限流和背压机制，防止慢客户端拖垮服务端。

### 6. 监控统计

添加连接数、收发字节数、RTT 分布等监控指标。

### 7. 集群扩展

结合服务发现（如 etcd、Consul）实现 TCP 服务集群和负载均衡。
