# pcap 网络数据包捕获模块 - 使用指南

## 模块概述

`src-tauri/src/pcap/` 是基于 [`pcap`](https://crates.io/crates/pcap) crate 封装的网络数据包捕获模块，提供与 C++ StreamPlayer 项目中 `capture/` 目录对等的处理能力。

### 核心功能

| 功能 | 对应 C++ StreamPlayer | Rust 实现 |
|------|----------------------|-------------|
| 网卡枚举 | `pcap_findalldevs()` → `getNetworkInterface()` | `list_devices()` |
| 打开网卡 | `pcap_open_live()` → `setCaptureHandler()` | `Capture::start_with_channel()` / `start_with_callback()` |
| 启动抓包循环 | `pcap_loop()` → 独立线程 `threadLoop()` | 独立线程 + `next_packet()` 循环 |
| 停止抓包 | `pcap_breakloop()` → `end()` | `Capture::stop()`（原子标志） |
| 数据包消费 | 静态回调 `packetHandle` + 全局队列 | 通道（`mpsc::Receiver`）或回调（`FnMut`） |

---

## 快速开始

### 1. 枚举网卡设备

```rust
use crate::pcap::device::list_devices;

let devices = list_devices().unwrap();
for dev in &devices {
    println!("网卡: {} - {}", 
        dev.name, 
        dev.description.as_deref().unwrap_or("(无描述)"));
}
```

### 2. 通道模式捕获（推荐）

通道模式是 Rust 中跨线程传递数据的惯用方式，替代 C++ 的 `std::queue` + `std::mutex` + `std::condition_variable` 组合。

```rust
use crate::pcap::capture::Capture;
use std::thread;

// 启动捕获（混杂模式，snaplen=65536，超时 1000ms）
let (mut capture, rx) = Capture::start_with_channel(
    r"\Device\NPF_{你的网卡GUID}",
    true,   // 混杂模式
    65536,  // snaplen
    1000,   // 超时 ms
).unwrap();

// 在独立线程中读取数据包
thread::spawn(move || {
    for pkt in rx {
        println!("收到数据包: {} 字节", pkt.data.len());
        // pkt.data 包含完整链路层帧（如 14 字节以太网头 + IP 包 + ...）
    }
});

// ... 等待一段时间后停止 ...
capture.stop();
```

### 3. 回调模式捕获

回调模式对标 C++ 的 `packetHandle` 静态回调函数。

```rust
use crate::pcap::capture::Capture;

let mut capture = Capture::start_with_callback(
    r"\Device\NPF_{你的网卡GUID}",
    true, 65536, 1000,
    |pkt: &[u8]| {
        // 应用层过滤：仅处理 EtherType == 0x22XX 的包（对标 C++ packet[12] == 0x22）
        if pkt.len() >= 14 {
            let ethertype_high = (pkt[12] as u16) >> 8;
            if ethertype_high == 0x22 {
                println!("收到自定义协议包，长度: {}", pkt.len());
            }
        }
    },
).unwrap();

// ... 等待一段时间后停止 ...
capture.stop();
```

---

## Windows 平台配置

### 安装 Npcap（运行时依赖）

1. 访问 [Npcap 官网](https://npcap.com/) 下载安装包
2. 安装时勾选 **"Install Npcap in WinPcap API-compatible Mode"**
3. 安装完成后，系统会存在 `wpcap.dll`（通常位于 `C:\Windows\System32\`）

### 安装 Npcap SDK（编译时依赖）

若直接编译出现链接错误，需要安装 Npcap SDK：

1. 访问 [Npcap SDK 下载页](https://npcap.com/#download)
2. 解压到任意目录（如 `C:\npcap-sdk\`）
3. 设置环境变量 `NPCAP_SDK_DIR` 指向 SDK 目录

```powershell
# PowerShell 中设置（永久）
[System.Environment]::SetEnvironmentVariable("NPCAP_SDK_DIR", "C:\npcap-sdk", "Machine")
```

### 管理员权限

混杂模式（`promisc: true`）抓包需要以管理员权限运行程序。

---

## 与 C++ StreamPlayer 的对应关系

### Capture 类成员对照

| C++ `Capture` 成员 | Rust `Capture` 成员 | 说明 |
|---------------------|---------------------|------|
| `captureHandler_` (`pcap_t*`) | （内部 `PcapCapture<Active>`） | Rust 中不直接暴露底层句柄 |
| `networkInterfaceList_` (`vector<string>`) | `list_devices()` 返回值 | 网卡枚举 |
| `s_packetArray` / `s_packetQueue` | `mpsc::Receiver<Packet>` | 数据包队列（Rust 用通道替代） |
| `start()` → `threadLoop()` | `start_with_channel()` / `start_with_callback()` | 启动捕获线程 |
| `end()` → `pcap_breakloop()` | `stop()` → 设置原子标志 | 停止捕获 |
| `packetHandle()` 静态回调 | 用户提供的 `FnMut(&[u8])` 闭包 | 数据包处理回调 |
| `isReady()` | `is_running()` | 查询状态（语义略有不同） |

### 数据包过滤逻辑迁移

C++ 中 `packetHandle` 的过滤逻辑：

```cpp
// capture.cpp:114
void Capture::packetHandle(unsigned char* userData, const struct pcap_pkthdr* header, const unsigned char* packet) {
    if ((packet[12] == 0x22)) {  // EtherType 高字节 == 0x22
        // ... 拷贝到环形缓冲 ...
    }
}
```

Rust 中对应的过滤逻辑（在回调中实现）：

```rust
Capture::start_with_callback(..., |pkt: &[u8]| {
    // pkt[12] 是以太网帧的第 12 字节（EtherType 高字节）
    if pkt.len() >= 14 && pkt[12] == 0x22 {
        // ... 处理数据包 ...
    }
});
```

> **注意**：pcap 模块作为通用封装，**不包含应用层过滤逻辑**，留给上层按需实现。

---

## 模块文件结构

```
src-tauri/src/pcap/
├── mod.rs        # 模块入口：声明子模块 + 重导出公共类型 + 模块文档
├── error.rs      # PcapError 枚举（thiserror 派生）
├── device.rs     # NetworkDevice 结构体 + list_devices() 网卡枚举
└── capture.rs    # Capture + Packet 结构体，核心捕获逻辑
```

### 公共 API 导出

```rust
// 错误类型
pub use crate::pcap::PcapError;

// 网卡设备
pub use crate::pcap::{list_devices, NetworkDevice};

// 捕获核心
pub use crate::pcap::{Capture, Packet};
```

---

## 错误处理

所有错误通过 `PcapError` 枚举返回，可使用 `?` 操作符传播：

```rust
use crate::pcap::PcapError;

fn setup_capture() -> Result<(), PcapError> {
    let devices = list_devices()?;  // 自动传播错误
    // ...
    Ok(())
}
```

错误变体说明：

| 变体 | 触发场景 |
|------|----------|
| `ListDevicesError(String)` | `pcap_findalldevs()` 失败（Npcap 未安装、权限不足） |
| `DeviceNotFound(String)` | （预留）未找到指定设备 |
| `OpenDeviceError { device, reason }` | `pcap_open_live()` 失败 |
| `NotReady` | （预留）捕获句柄未就绪 |
| `AlreadyRunning` | （预留）重复启动捕获 |
| `NotRunning` | （预留）捕获未运行 |
| `CaptureError(String)` | 捕获循环中发生错误 |
| `SetFilterError(String)` | 设置 BPF 过滤器失败 |

---

## 示例代码：完整的抓包流程

```rust
use crate::pcap::{list_devices, Capture, Packet};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 枚举网卡
    let devices = list_devices()?;
    if devices.is_empty() {
        eprintln!("未找到任何网卡设备");
        return Ok(());
    }

    // 2. 选择第一个网卡（实际使用中应由用户选择）
    let device = &devices[0];
    println!("使用网卡: {} - {}", 
        device.name, 
        device.description.as_deref().unwrap_or(""));

    // 3. 启动捕获（通道模式）
    let (mut capture, rx) = Capture::start_with_channel(
        &device.name,
        true,   // 混杂模式
        65536,  // snaplen
        1000,   // 超时 ms
    )?;

    // 4. 在独立线程中处理数据包
    let handle = thread::spawn(move || {
        let mut count = 0;
        for pkt in rx {
            count += 1;
            if count % 100 == 0 {
                println!("已收到 {} 个数据包", count);
            }
        }
        println!("捕获结束");
    });

    // 5. 等待一段时间
    thread::sleep(Duration::from_secs(10));

    // 6. 停止捕获
    capture.stop();

    // 7. 等待处理线程结束
    handle.join().unwrap();

    Ok(())
}
```

---

## 常见问题

### Q: 编译时报错 "cannot find -lwpcap" 或类似链接错误？

**A**: Windows 上需要安装 Npcap SDK 并设置 `NPCAP_SDK_DIR` 环境变量。参见上方「Windows 平台配置」章节。

### Q: 运行时报错 "failed to open device: ... Permission denied"？

**A**: 需要以管理员权限运行程序（尤其在使用混杂模式时）。

### Q: 如何设置 BPF 过滤器（如仅捕获 UDP 包）？

**A**: 当前版本未封装 `pcap_setfilter()`，如有需要可自行调用 `pcap` crate 的底层 API：

```rust
use pcap::Capture;

// 在 start_with_* 之前，通过修改 spawn_capture_thread 内部逻辑来支持
// （后续版本可考虑封装 filter 参数）
```

### Q: 数据包的 `data` 字段包含什么内容？

**A**: `data` 包含从链路层开始的完整帧。对于以太网：
- 字节 0-5：目标 MAC 地址
- 字节 6-11：源 MAC 地址
- 字节 12-13：EtherType（如 `0x0800` = IPv4，`0x22XX` = 自定义协议）
- 字节 14+： payload（IP 包等）

---

## 参考资料

- [pcap crate - crates.io](https://crates.io/crates/pcap)
- [pcap crate - docs.rs](https://docs.rs/pcap/latest/pcap/)
- [Npcap 官网](https://npcap.com/)
- [Libpcap 官方文档](https://www.tcpdump.org/)
