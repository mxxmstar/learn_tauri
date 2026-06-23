# UDP 模块使用说明

本模块基于 `tokio::net::UdpSocket` 实现完整的异步 UDP 通信功能，包括服务端和客户端，支持结构化消息（JSON）和原始字节传输。

## 目录

- [模块结构](#模块结构)
- [核心特性](#核心特性)
- [快速开始](#快速开始)
  - [服务端示例](#服务端示例)
  - [客户端示例](#客户端示例)
  - [回声服务器示例](#回声服务器示例)
- [配置说明](#配置说明)
  - [UdpServerConfig](#udpserverconfig)
  - [UdpClientConfig](#udpclientconfig)
  - [MulticastConfig](#multicastconfig)
- [消息协议](#消息协议)
  - [MessageType 消息类型](#messagetype-消息类型)
  - [UdpMessage 结构](#udpmessage-结构)
- [API 参考](#api-参考)
  - [UdpServer](#udpserver)
  - [UdpClient](#udpclient)
  - [UdpService](#udpservice)
- [典型应用场景](#典型应用场景)
  - [场景一：简单请求-响应](#场景一简单请求-响应)
  - [场景二：服务端推送](#场景二服务端推送)
  - [场景三：广播通信](#场景三广播通信)
  - [场景四：多播通信](#场景四多播通信)
  - [场景五：心跳检测](#场景五心跳检测)
- [注意事项](#注意事项)
- [扩展建议](#扩展建议)

---

## 模块结构

```
udp/
├── mod.rs      # 模块主入口，定义 UdpService 统一管理接口
├── config.rs   # 配置管理（服务端/客户端配置、多播配置）
├── message.rs  # 消息协议定义与编解码（JSON 结构化消息）
├── server.rs   # UDP 异步服务端
├── client.rs   # UDP 异步客户端
└── README.md   # 使用说明文档（本文件）
```

### 各文件职责

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块入口，重新导出类型，提供 `UdpService` 统一管理接口 |
| `config.rs` | 定义 `UdpServerConfig`、`UdpClientConfig`、`MulticastConfig`，包含配置验证逻辑 |
| `message.rs` | 定义 `MessageType` 和 `UdpMessage`，实现 JSON 编解码 |
| `server.rs` | 实现 `UdpServer`，异步接收循环、回调机制、广播/多播支持 |
| `client.rs` | 实现 `UdpClient`，连接、收发、Ping/Pong、后台接收循环 |

---

## 核心特性

- **异步 IO**：基于 `tokio::net::UdpSocket`，全异步非阻塞，适合高并发场景
- **结构化消息**：使用 JSON 编码的 `UdpMessage`，支持消息类型区分（Data/Broadcast/Ping/Pong/Custom）
- **心跳检测**：内置 Ping/Pong 机制，支持 RTT（往返延迟）测量
- **广播与多播**：支持 UDP 广播（`255.255.255.255`）和多播组通信（`224.0.0.0/4`）
- **回调机制**：服务端支持注册消息回调和原始字节回调，灵活处理业务逻辑
- **超时控制**：客户端支持可配置的读写超时（`timeout_ms`）
- **优雅停机**：通过 `tokio::sync::watch` 通道实现服务端/客户端的优雅停止
- **配置验证**：所有配置在创建时自动验证，避免运行时错误

---

## 快速开始

### 服务端示例

```rust
use learn_tauri_lib::udp::{UdpService, UdpServerConfig, UdpMessage};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. 创建服务端配置
    let config = UdpServerConfig::new("0.0.0.0:8080")
        .with_buffer_size(8192)
        .with_broadcast(true);

    // 2. 创建服务
    let mut service = UdpService::new_server(config)?;

    // 3. 设置消息回调
    service.on_message(|msg, addr| {
        println!("[服务端] 收到来自 {} 的消息: {}", addr, msg.payload);
        Box::pin(async {})
    }).await?;

    // 4. 启动服务
    service.start_server().await?;
    println!("UDP 服务端已启动，监听 0.0.0.0:8080");

    // 5. 保持运行
    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;

    // 6. 停止服务
    service.stop_server().await
}
```

### 客户端示例

```rust
use learn_tauri_lib::udp::{UdpService, UdpClientConfig, UdpMessage};

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. 创建客户端配置
    let config = UdpClientConfig::new("127.0.0.1:8080")
        .with_timeout(3000);  // 3 秒超时

    // 2. 创建服务
    let service = UdpService::new_client(config)?;

    // 3. 连接服务端
    service.connect().await?;

    // 4. 发送消息
    service.send(&UdpMessage::data("Hello, Server!")).await?;
    println!("[客户端] 已发送消息");

    // 5. 接收回复
    let (msg, addr) = service.recv().await?;
    println!("[客户端] 收到来自 {} 的回复: {}", addr, msg.payload);

    // 6. Ping 测试
    let rtt = service.ping().await?;
    println!("[客户端] RTT: {}ms", rtt);

    Ok(())
}
```

### 回声服务器示例

回声服务器收到任何消息后原样返回，常用于测试网络连通性：

```rust
use learn_tauri_lib::udp::{UdpServer, UdpServerConfig, UdpMessage};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = UdpServerConfig::new("0.0.0.0:8080");
    let mut server = UdpServer::new(config)?;

    // 设置回声回调：收到消息后原样发回
    server.on_message(|msg, addr| {
        let echo = UdpMessage::data(format!("ECHO: {}", msg.payload));
        Box::pin(async move {
            println!("[Echo] 回声消息到 {}", addr);
            // 注意：在回调中需要通过其他方式访问 socket 发送
            // 推荐使用 UdpService::server_send 或共享 socket
        })
    }).await;

    server.start().await?;

    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;
    Ok(())
}
```

---

## 配置说明

### UdpServerConfig

服务端配置。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `listen_addr` | `String` | `"0.0.0.0:8080"` | 监听地址（`IP:Port` 格式） |
| `buffer_size` | `usize` | `4096` | 接收缓冲区大小（字节） |
| `broadcast` | `bool` | `false` | 是否启用广播模式 |
| `multicast` | `Option<MulticastConfig>` | `None` | 多播配置（可选） |

**构建方法（链式调用）：**

```rust
let config = UdpServerConfig::new("0.0.0.0:8080")
    .with_buffer_size(8192)           // 设置缓冲区大小
    .with_broadcast(true)             // 启用广播
    .with_multicast(mc_config);       // 启用多播
```

### UdpClientConfig

客户端配置。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `server_addr` | `String` | `"127.0.0.1:8080"` | 目标服务端地址 |
| `bind_addr` | `Option<String>` | `None` | 本地绑定地址（`None` 表示系统自动分配） |
| `buffer_size` | `usize` | `4096` | 接收缓冲区大小（字节） |
| `timeout_ms` | `u64` | `5000` | 读写超时时间（毫秒，`0` 表示不超时） |

**构建方法（链式调用）：**

```rust
let config = UdpClientConfig::new("127.0.0.1:8080")
    .with_bind_addr("0.0.0.0:12345")  // 固定本地端口
    .with_buffer_size(8192)            // 设置缓冲区大小
    .with_timeout(3000);               // 3 秒超时
```

### MulticastConfig

多播配置。

| 字段 | 类型 | 说明 |
|------|------|------|
| `group_addr` | `String` | 多播组地址（`224.0.0.0` ~ `239.255.255.255`） |
| `interface` | `Option<String>` | 多播接口（`None` 表示使用默认接口） |
| `ttl` | `u32` | 多播 TTL（生存时间，控制传播跳数） |

**示例：**

```rust
use learn_tauri_lib::udp::{UdpServerConfig, MulticastConfig};

let mc = MulticastConfig {
    group_addr: "239.0.0.1".to_string(),
    interface: None,
    ttl: 4,
};

let config = UdpServerConfig::new("0.0.0.0:8080")
    .with_multicast(mc);
```

---

## 消息协议

本模块使用 JSON 编码的结构化消息，便于解析和扩展。

### MessageType 消息类型

| 类型 | 说明 | 使用场景 |
|------|------|----------|
| `Data` | 普通数据消息 | 日常通信 |
| `Broadcast` | 广播消息 | 广播通知 |
| `Ping` | 心跳请求 | 连通性检测 |
| `Pong` | 心跳响应 | 回复 Ping（服务端自动回复） |
| `Custom(String)` | 自定义类型 | 业务特定消息 |

### UdpMessage 结构

```json
{
    "msg_type": "Data",
    "payload": "Hello, UDP!",
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
let msg = UdpMessage::data("Hello");

// 广播消息
let msg = UdpMessage::broadcast("通知所有人");

// Ping 消息
let ping = UdpMessage::ping();

// Pong 消息（回复 Ping，传入原 Ping 的时间戳）
let pong = UdpMessage::pong(ping.timestamp);

// 自定义类型消息
let msg = UdpMessage::custom("Login", "{\"user\":\"alice\"}");
```

---

## API 参考

### UdpServer

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建服务端实例 |
| `on_message(callback)` | 设置结构化消息回调 |
| `on_raw(callback)` | 设置原始字节回调 |
| `start()` | 启动服务端（非阻塞） |
| `stop()` | 停止服务端 |
| `send(msg, addr)` | 发送消息到指定地址 |
| `broadcast(msg, addr)` | 广播消息 |
| `local_addr()` | 获取本地绑定地址 |
| `config()` | 获取配置引用 |

### UdpClient

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建客户端实例 |
| `connect()` | 连接服务端（关联地址） |
| `disconnect()` | 断开连接 |
| `send(msg)` | 发送结构化消息 |
| `send_raw(data)` | 发送原始字节 |
| `recv()` | 接收消息（阻塞，支持超时） |
| `recv_raw()` | 接收原始字节（阻塞，支持超时） |
| `ping()` | Ping 测试，返回 RTT（毫秒） |
| `request(msg)` | 请求-响应模式 |
| `start_recv_loop()` | 启动后台接收循环 |
| `local_addr()` | 获取本地地址 |
| `peer_addr()` | 获取服务端地址 |
| `is_connected()` | 是否已连接 |
| `config()` | 获取配置引用 |

### UdpService

统一管理接口，自动判断服务端/客户端模式。

| 方法 | 说明 |
|------|------|
| `new_server(config)` | 创建服务端模式 |
| `new_client(config)` | 创建客户端模式 |
| `on_message(cb)` | 设置消息回调（服务端） |
| `on_raw(cb)` | 设置原始回调（服务端） |
| `start_server()` | 启动服务端 |
| `stop_server()` | 停止服务端 |
| `server_send(msg, addr)` | 服务端发送 |
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

### 场景一：简单请求-响应

客户端发送请求，服务端处理后返回响应。

**服务端：**

```rust
use learn_tauri_lib::udp::{UdpServer, UdpServerConfig, UdpMessage};

let config = UdpServerConfig::new("0.0.0.0:8080");
let mut server = UdpServer::new(config)?;

server.on_message(|msg, addr| {
    let payload = msg.payload.clone();
    Box::pin(async move {
        // 处理请求并准备响应
        let response = format!("处理结果: {}", payload);
        println!("处理来自 {} 的请求: {}", addr, payload);
        // 通过共享 socket 发送响应（需在回调外持有 socket 引用）
    })
}).await;

server.start().await?;
```

**客户端：**

```rust
use learn_tauri_lib::udp::{UdpClient, UdpClientConfig, UdpMessage};

let config = UdpClientConfig::new("127.0.0.1:8080").with_timeout(3000);
let client = UdpClient::new(config)?;
client.connect().await?;

// 请求-响应模式
let (response, addr) = client.request(&UdpMessage::data("查询用户列表")).await?;
println!("收到响应: {}", response.payload);
```

### 场景二：服务端推送

服务端主动向客户端推送消息，客户端使用后台接收循环。

**客户端：**

```rust
use learn_tauri_lib::udp::{UdpClient, UdpClientConfig};

let config = UdpClientConfig::new("127.0.0.1:8080");
let client = UdpClient::new(config)?;
client.connect().await?;

// 启动后台接收循环
let mut rx = client.start_recv_loop().await?;

// 在另一个任务中处理收到的消息
tokio::spawn(async move {
    while let Some((msg, addr)) = rx.recv().await {
        println!("[推送] 来自 {}: {}", addr, msg.payload);
    }
});

// 主线程可以继续发送消息
client.send(&UdpMessage::data("订阅通知")).await?;
```

### 场景三：广播通信

服务端向所有客户端广播消息。

**服务端：**

```rust
use learn_tauri_lib::udp::{UdpServer, UdpServerConfig, UdpMessage};
use std::net::SocketAddr;

let config = UdpServerConfig::new("0.0.0.0:8080").with_broadcast(true);
let mut server = UdpServer::new(config)?;
server.start().await?;

// 广播消息到 255.255.255.255:9090
let broadcast_addr: SocketAddr = "255.255.255.255:9090".parse().unwrap();
server.broadcast(&UdpMessage::broadcast("系统通知：5分钟后维护"), broadcast_addr).await?;
```

**客户端（监听广播）：**

```rust
use learn_tauri_lib::udp::{UdpClient, UdpClientConfig};
// 客户端需要绑定到广播目标端口
let config = UdpClientConfig::new("0.0.0.0:9090")  // 指向广播源
    .with_bind_addr("0.0.0.0:9090");                // 监听 9090 端口
let client = UdpClient::new(config)?;
client.connect().await?;

let mut rx = client.start_recv_loop().await?;
while let Some((msg, addr)) = rx.recv().await {
    println!("[广播] 来自 {}: {}", addr, msg.payload);
}
```

### 场景四：多播通信

多个客户端加入同一多播组，服务端向多播组发送消息。

**服务端：**

```rust
use learn_tauri_lib::udp::{UdpServer, UdpServerConfig, MulticastConfig, UdpMessage};
use std::net::SocketAddr;

let mc = MulticastConfig {
    group_addr: "239.0.0.1".to_string(),
    interface: None,
    ttl: 4,
};

let config = UdpServerConfig::new("0.0.0.0:8080").with_multicast(mc);
let mut server = UdpServer::new(config)?;
server.start().await?;

// 向多播组发送消息
let group_addr: SocketAddr = "239.0.0.1:8080".parse().unwrap();
server.send(&UdpMessage::broadcast("多播通知"), group_addr).await?;
```

### 场景五：心跳检测

客户端定期发送 Ping，服务端自动回复 Pong，用于检测连接存活和测量延迟。

```rust
use learn_tauri_lib::udp::{UdpClient, UdpClientConfig};
use std::time::Duration;

let config = UdpClientConfig::new("127.0.0.1:8080").with_timeout(2000);
let client = UdpClient::new(config)?;
client.connect().await?;

// 每 10 秒发送一次 Ping
loop {
    match client.ping().await {
        Ok(rtt) => println!("[心跳] 连接正常，RTT: {}ms", rtt),
        Err(e) => {
            println!("[心跳] 连接异常: {}", e);
            break;
        }
    }
    tokio::time::sleep(Duration::from_secs(10)).await;
}
```

---

## 注意事项

### 1. UDP 协议特性

- **无连接**：UDP 不保证数据到达、顺序或去重，适合实时性要求高、容错性强的场景
- **数据报边界**：每次 `recv_from` 读取一个完整数据报，不会跨包
- **最大数据报大小**：建议单包不超过 MTU（通常 1472 字节），避免 IP 分片

### 2. 权限要求

- 绑定 1024 以下的端口（特权端口）需要管理员权限
- Windows 防火墙可能阻止 UDP 通信，需添加例外规则

### 3. 广播与多播

- **广播**：只能发送到本地子网，目标地址为 `255.255.255.255`
- **多播**：可跨子网（需路由器支持），地址范围为 `224.0.0.0` ~ `239.255.255.255`
- 多播 TTL 控制传播范围：
  - `0`：仅限本机
  - `1`：仅限本子网
  - `>1`：可跨子网（需路由器转发）

### 4. 超时处理

- 客户端的 `timeout_ms` 仅作用于 `recv()`、`recv_raw()` 和 `ping()` 方法
- `send()` 通常不超时（UDP 发送不阻塞）
- 超时设置为 `0` 表示永不超时（一直阻塞等待）

### 5. 并发安全

- `UdpClient` 实现了 `Clone`，可在多个异步任务间共享
- `UdpServer` 内部使用 `Arc` 共享套接字，但结构体本身不可克隆（需 `&mut` 操作）
- 回调函数需满足 `Send + Sync + 'static` 约束

### 6. 在 Tauri 中使用

在 Tauri 命令中使用 UDP 时，建议：

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

// 使用全局状态保存 UdpService
struct AppState {
    udp_service: Arc<Mutex<Option<udp::UdpService>>>,
}

#[tauri::command]
async fn start_udp_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut service = state.udp_service.lock().await;
    let mut new_service = udp::UdpService::new_server(
        udp::UdpServerConfig::new("0.0.0.0:8080")
    )?;
    new_service.start_server().await?;
    *service = Some(new_service);
    Ok(())
}
```

---

## 扩展建议

如需增强功能，可考虑以下扩展：

### 1. 可靠性传输

UDP 本身不可靠，如需可靠性，可在此基础上实现：
- 序列号与 ACK 确认机制
- 超时重传
- 滑动窗口流控

### 2. 数据分片

对于超过 MTU 的大消息，可实现分片传输：
- 将大消息拆分为多个小数据报
- 添加序号和总数字段
- 接收端重组

### 3. 加密通信

使用 `ring` 或 `aes-gcm` 库对 payload 加密，确保通信安全。

### 4. 连接管理

实现客户端连接池、会话管理和鉴权机制。

### 5. 监控统计

添加收发包计数、丢包率统计、延迟监控等指标。

### 6. 多协议支持

扩展 `MessageType`，支持更多业务消息类型，或支持 Protocol Buffers 等高效编码格式。
