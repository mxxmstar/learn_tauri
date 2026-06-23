# DHCP 服务器模块使用说明

## 模块结构

本模块实现了一个完整的 DHCP 服务器功能，采用分文件设计：

```
dhcp/
├── mod.rs      # 模块主入口，定义 DhcpService
├── config.rs   # 配置管理（网段、地址范围等）
├── pool.rs     # IP 地址池管理
├── lease.rs    # 租约管理
├── protocol.rs # DHCP 协议消息处理
└── server.rs   # DHCP 服务器主逻辑
```

## 快速开始

### 1. 基本使用

```rust
use crate::dhcp::{DhcpService, init_dhcp_service};

// 方法一：使用便捷函数初始化
let dhcp_service = init_dhcp_service(
    "192.168.1.0/24",                      // 网段
    "192.168.1.1",                         // 网关
    vec!["8.8.8.8".to_string()],          // DNS 服务器
    vec![                                   // 地址池范围
        ("192.168.1.100".to_string(), "192.168.1.200".to_string()),
    ],
)?;

// 方法二：手动创建配置
use crate::dhcp::{DhcpConfig, DhcpPoolRange};

let config = DhcpConfig {
    subnet: "192.168.1.0/24".to_string(),
    gateway: "192.168.1.1".to_string(),
    dns_servers: vec!["8.8.8.8".to_string(), "114.114.114.114".to_string()],
    pools: vec![
        DhcpPoolRange {
            start_ip: "192.168.1.100".to_string(),
            end_ip: "192.168.1.200".to_string(),
        },
    ],
    lease_time_default: 86400,  // 默认 24 小时
    lease_time_max: 604800,     // 最大 7 天
    listen_port: 67,           // DHCP 服务器端口
    interface: None,           // 网络接口（可选）
};

let dhcp_service = DhcpService::new(config)?;
```

### 2. 启动 DHCP 服务器

```rust
// 注意：启动服务器需要在 async 上下文中
// 可以使用 tokio 运行时

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut dhcp_service = init_dhcp_service(
        "192.168.1.0/24",
        "192.168.1.1",
        vec!["8.8.8.8".to_string()],
        vec![("192.168.1.100".to_string(), "192.168.1.200".to_string())],
    )?;
    
    // 启动 DHCP 服务器
    dhcp_service.start().await?;
    
    println!("DHCP 服务器已启动");
    
    // 保持运行
    tokio::signal::ctrl_c().await.unwrap();
    
    // 停止 DHCP 服务器
    dhcp_service.stop().await?;
    
    Ok(())
}
```

### 3. 查询服务器状态

```rust
// 获取地址池统计信息
let (allocated, available) = dhcp_service.get_pool_stats().await;
println!("已分配: {} 个地址", allocated);
println!("可用: {} 个地址", available);

// 获取所有租约
let leases = dhcp_service.get_leases().await;
for lease in leases {
    println!("IP: {}, MAC: {}, 状态: {:?}", 
             lease.ip_address, 
             lease.mac_address, 
             lease.state);
}
```

## 配置说明

### DhcpConfig 配置项

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `subnet` | String | 网段（CIDR 格式） | "192.168.1.0/24" |
| `gateway` | String | 网关地址 | "192.168.1.1" |
| `dns_servers` | Vec<String> | DNS 服务器列表 | ["8.8.8.8", "114.114.114.114"] |
| `pools` | Vec<DhcpPoolRange> | 地址池范围列表 | 见下方 |
| `lease_time_default` | u32 | 默认租约时间（秒） | 86400 (24小时) |
| `lease_time_max` | u32 | 最大租约时间（秒） | 604800 (7天) |
| `listen_port` | u16 | 监听端口 | 67 (DHCP 标准端口) |
| `interface` | Option<String> | 网络接口 | None (表示所有接口) |

### DhcpPoolRange 配置项

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `start_ip` | String | 起始 IP 地址 | "192.168.1.100" |
| `end_ip` | String | 结束 IP 地址 | "192.168.1.200" |

## 功能特性

### 1. 配置管理 (config.rs)

- 支持完整的 DHCP 服务器配置
- 配置验证功能
- IP 地址和网段格式验证

### 2. IP 地址池管理 (pool.rs)

- 自动分配 IP 地址
- 支持同一 MAC 地址重复分配同一 IP
- IP 地址回收和释放
- 地址池统计信息

### 3. 租约管理 (lease.rs)

- 创建、续期、释放租约
- 租约过期自动检测
- 租约清理功能
- 租约统计信息

### 4. 协议处理 (protocol.rs)

- DHCP 消息编码/解码
- 支持主要 DHCP 消息类型：
  - DISCOVER
  - OFFER
  - REQUEST
  - ACK
  - NAK
  - RELEASE
  - DECLINE
  - INFORM

### 5. 服务器逻辑 (server.rs)

- UDP 套接字监听
- 异步消息处理
- 广播响应支持

## 注意事项

1. **权限要求**：DHCP 服务器需要绑定到 67 端口（特权端口），可能需要管理员权限运行。

2. **网络配置**：确保网络接口配置正确，特别是防火墙规则。

3. **异步运行时**：本模块使用 tokio 作为异步运行时，确保在 `Cargo.toml` 中添加了相应依赖。

4. **生产环境**：本实现为学习和演示目的，生产环境建议使用成熟的 DHCP 服务器软件（如 ISC DHCP、dnsmasq 等）。

## 扩展建议

如需在生产环境使用，建议扩展以下功能：

1. **持久化存储**：将租约信息保存到数据库或文件
2. **静态 IP 分配**：支持根据 MAC 地址分配固定 IP
3. **DHCP 选项**：支持更多 DHCP 选项（如 NTP、WINS 等）
4. **高可用性**：支持 DHCP 服务器集群和故障转移
5. **Web 管理界面**：提供 Web UI 进行配置和管理
6. **日志记录**：增强日志记录和审计功能
7. **安全特性**：支持 DHCP Snooping、IP Source Guard 等安全特性

## 示例代码

完整的示例代码请参考 `examples/dhcp_server_example.rs`（待创建）。
