//! # pcap 网络数据包捕获模块
//!
//! 封装 [`pcap`] crate（libpcap/Npcap 的 Rust 安全绑定），
//! 提供网卡枚举、实时数据包捕获等能力，
//! 对标 C++ 项目中 `StreamPlayer/capture/` 的功能。
//!
//! ## 模块结构
//!
//! | 文件 | 说明 |
//! |------|------|
//! | `error.rs` | `PcapError` 枚举，统一错误类型 |
//! | `device.rs` | `NetworkDevice` 结构体 + `list_devices()` 网卡枚举 |
//! | `capture.rs` | `Capture` + `Packet` 结构体，核心捕获逻辑 |
//!
//! ## 快速开始
//!
//! ### 1. 枚举网卡
//!
//! ```rust
//! use crate::pcap::device::list_devices;
//!
//! let devices = list_devices().unwrap();
//! for dev in &devices {
//!     println!("网卡: {} - {}", dev.name, dev.description.as_deref().unwrap_or(""));
//! }
//! ```
//!
//! ### 2. 通道模式捕获
//!
//! ```rust
//! use crate::pcap::capture::Capture;
//!
//! let (mut capture, rx) = Capture::start_with_channel(
//!     r"\Device\NPF_{GUID}",
//!     true,   // 混杂模式
//!     65536,  // snaplen
//!     1000,   // 超时 ms
//! ).unwrap();
//!
//! // 在独立线程中读取数据包
//! std::thread::spawn(move || {
//!     for pkt in rx {
//!         println!("收到 {} 字节", pkt.data.len());
//!     }
//! });
//!
//! // ... 等待 ...
//! capture.stop();
//! ```
//!
//! ### 3. 回调模式捕获
//!
//! ```rust
//! use crate::pcap::capture::Capture;
//!
//! let mut capture = Capture::start_with_callback(
//!     r"\Device\NPF_{GUID}",
//!     true, 65536, 1000,
//!     |pkt: &[u8]| {
//!         // 应用层过滤（对标 C++ packet[12] == 0x22）
//!         if pkt.len() >= 14 && u16::from_be_bytes([pkt[12], pkt[13]]) >> 8 == 0x22 {
//!             println!("收到自定义协议包");
//!         }
//!     },
//! ).unwrap();
//!
//! // ... 等待 ...
//! capture.stop();
//! ```
//!
//! ## Windows 平台注意事项
//!
//! - **运行时依赖**：需安装 [Npcap](https://npcap.com/)（勾选 "Install Npcap in WinPcap API-compatible Mode"）
//! - **编译时依赖**：需安装 Npcap SDK，并设置环境变量 `NPCAP_SDK_DIR` 指向 SDK 目录
//! - **权限**：混杂模式抓包需要以管理员权限运行程序

// ============================================================
// 子模块声明
// ============================================================

pub mod error;
pub mod device;
pub mod capture;

// ============================================================
// 公共类型重导出（方便调用方使用）
// ============================================================

// 错误类型
pub use error::PcapError;

// 网卡设备
pub use device::{list_devices, NetworkDevice};

// 捕获核心
pub use capture::{Capture, Packet};
