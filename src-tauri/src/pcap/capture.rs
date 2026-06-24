//! 网络数据包捕获核心实现
//!
//! 封装 `pcap::Capture`，提供两种数据消费模式：
//!
//! 1. **通道模式**（`start_with_channel`）：返回 `mpsc::Receiver<Packet>`，
//!    调用方通过接收端按需读取数据包，是 Rust 中跨线程传递数据的惯用方式。
//! 2. **回调模式**（`start_with_callback`）：每收到一个包调用用户提供的闭包，
//!    对标 C++ `Capture` 类中的 `packetHandle` 回调函数。
//!
//! 捕获在独立线程中运行，通过 `Arc<AtomicBool>` 原子标志控制启停，
//! 替代 C++ 中的 `pcap_breakloop()`` + `pcap_close()` 组合。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::SystemTime;

// 使用别名避免与自定义的 `Capture` 结构体冲突
use pcap::Capture as PcapCapture;
use pcap::Device;

use crate::pcap::error::PcapError;

// ============================================================
// 数据包结构体
// ============================================================

/// 表示一个捕获到的网络数据包（拥有所有权）。
///
/// 与 `pcap::Packet` 不同，此结构体**拥有**数据缓冲区（`Vec<u8>`），
/// 可以安全地跨线程传递，不受底层 `pcap` 捕获句柄生命周期的限制。
/// 对标 C++ 中拷贝到环形缓冲区的 `unsigned char packet[]`。
#[derive(Debug, Clone)]
pub struct Packet {
    /// 数据包被捕获的时间戳（系统时间）。
    pub timestamp: SystemTime,

    /// 实际捕获到的字节数（可能小于线路上的原始长度，受 snaplen 限制）。
    pub caplen: u32,

    /// 线路上该数据包的原始长度（不含捕获截断）。
    pub len: u32,

    /// 数据包的原始字节内容（包含链路层头，如 Ethernet 帧头）。
    ///
    /// 对于 StreamPlayer 的场景，以太网帧的前 12 字节是 MAC 地址，
    /// 第 12-13 字节是 EtherType（如 `0x22XX` 表示自定义协议）。
    pub data: Vec<u8>,
}

impl Packet {
    /// 从 `pcap::Packet` 构造一个拥有所有权的 `Packet`。
    ///
    /// 会拷贝数据包内容到新的 `Vec<u8>`，代价为 O(caplen)。
    /// 与 C++ 中 `memcpy(s_packetArray[index], packet, header->caplen)` 等价。
    fn from_pcap(pcap_pkt: &pcap::Packet) -> Self {
        // pcap::Packet 的 header 字段包含时间戳、caplen、len
        // 注意：header 是字段不是方法，直接用 `&pcap_pkt.header` 访问
        let header = &pcap_pkt.header;

        // 将 pcap 的时间戳（timeval）转换为 SystemTime
        // pcap crate 的 PacketHeader 中 ts 字段类型为 pcap::TimeVal { tv_sec, tv_usec }
        let timestamp = SystemTime::UNIX_EPOCH
            + std::time::Duration::new(header.ts.tv_sec as u64, header.ts.tv_usec as u32 * 1000);

        Self {
            timestamp,
            caplen: header.caplen,
            len: header.len,
            // pcap::Packet 实现了 Deref<Target=[u8]>，通过 &[u8] 拷贝数据
            data: Vec::from(&pcap_pkt as &[u8]),
        }
    }
}

// ============================================================
// 捕获结构体
// ============================================================

/// 网络数据包捕获器。
///
/// 封装 `pcap` 的底层捕获循环，在独立线程中运行，
/// 支持通过**通道**或**回调**两种方式消费数据包。
///
/// # 启停控制
/// 内部使用 `Arc<AtomicBool>` 作为停止标志，
/// 捕获线程每次 `next_packet()` 超时后检查此标志，
/// 替代 C++ 中的 `pcap_breakloop()` + `pcap_close()` 组合。
///
/// # 自动清理
/// 当 `Capture` 被丢弃时（离开作用域），`Drop` trait 会自动调用 `stop()`，
/// 确保捕获线程和资源被正确释放。
pub struct Capture {
    /// 原子停止标志，捕获线程和调用方共享。
    ///
    /// - `true`：捕获线程继续运行
    /// - `false`：捕获线程应在下一次超时后退出
    running: Arc<AtomicBool>,

    /// 捕获线程的 JoinHandle。
    ///
    /// 当 `running` 设为 `false` 后，调用 `stop()` 会对此 handle 执行 `join()`，
    /// 确保线程完全退出后再返回。
    thread_handle: Option<JoinHandle<()>>,
}

impl Capture {
    // --------------------------------------------------------
    // 公共 API：启动捕获（通道模式）
    // --------------------------------------------------------

    /// 以**通道模式**启动数据包捕获。
    ///
    /// 在独立线程中打开指定网卡并建立捕获循环，
    /// 每捕获一个包即通过 `mpsc::Sender` 发送到通道中，
    /// 调用方通过返回的 `Receiver` 读取数据包。
    ///
    /// 对标 C++ `Capture::start()` + `packetHandle()` 中的队列推送逻辑，
    /// 但使用 Rust 的 `mpsc` 通道替代 `std::queue` + mutex + condvar。
    ///
    /// # 参数
    /// - `device_name`：网卡设备名称（来自 `list_devices()` 的 `NetworkDevice::name`）
    /// - `promisc`：是否开启混杂模式（`true` 表示接收所有经过网卡的数据包，
    ///   不仅限于发往本机的；对应 C++ 中 `pcap_open_live()` 的第 3 个参数）
    /// - `snaplen`：单个数据包最大捕获字节数（对应 C++ 中的 `65536`）
    /// - `timeout_ms`：`next_packet()` 的超时时间（毫秒），也是 `stop()` 的最大等待时间
    ///
    /// # 返回值
    /// 成功时返回 `(Capture, Receiver<Packet>)`：
    /// - `Capture`：捕获器句柄，用于后续调用 `stop()`
    /// - `Receiver<Packet>`：数据包接收端，调用方通过 `recv()` 或 `try_recv()` 读取
    ///
    /// # 错误
    /// - `PcapError::OpenDeviceError`：打开设备失败（权限不足、Npcap 未安装等）
    ///
    /// # 示例
    /// ```
    /// let (mut capture, rx) = Capture::start_with_channel(
    ///     r"\Device\NPF_{GUID}",
    ///     true,   // 混杂模式
    ///     65536,  // snaplen
    ///     1000,   // 超时 1000ms
    /// ).unwrap();
    ///
    /// // 在主线程或独立线程中读取数据包
    /// for pkt in rx {
    ///     println!("收到 {} 字节的数据包", pkt.data.len());
    /// }
    ///
    /// // 停止捕获
    /// capture.stop();
    /// ```
    pub fn start_with_channel(
        device_name: &str,
        promisc: bool,
        snaplen: i32,
        timeout_ms: i32,
    ) -> Result<(Self, mpsc::Receiver<Packet>), PcapError> {
        // 创建 mpsc 通道，捕获线程为发送端，调用方为接收端
        let (tx, rx) = mpsc::channel();

        // 创建原子停止标志（调用方和捕获线程共享）
        let running = Arc::new(AtomicBool::new(true));

        // 启动捕获线程（传入 running 的克隆）
        // 注意：device_name 是 &str，需要转为 String（spawn_capture_thread 要求 String）
        let thread_handle = Self::spawn_capture_thread(
            device_name.to_string(),
            promisc,
            snaplen,
            timeout_ms,
            Some(tx),
            None,
            running.clone(),
        )?;

        let capture = Self {
            running,
            thread_handle: Some(thread_handle),
        };

        Ok((capture, rx))
    }

    // --------------------------------------------------------
    // 公共 API：启动捕获（回调模式）
    // --------------------------------------------------------

    /// 以**回调模式**启动数据包捕获。
    ///
    /// 在独立线程中打开指定网卡并建立捕获循环，
    /// 每捕获一个包即调用 `callback(&[u8])`，
    /// 对标 C++ `Capture` 类中的 `packetHandle` 静态回调函数。
    ///
    /// # 参数
    /// - `device_name`、`promisc`、`snaplen`、`timeout_ms`：含义同 `start_with_channel()`
    /// - `callback`：数据包回调函数，接收原始数据包字节切片 `&[u8]`
    ///   （包含链路层头，如 14 字节以太网帧头）
    ///
    /// # 注意事项
    /// - `callback` 在捕获线程中执行，应避免执行耗时操作，否则会丢包。
    /// - 若需跨线程传递数据，建议在 `callback` 中拷贝数据到通道或队列。
    /// - 应用层过滤逻辑（如 C++ 中的 `packet[12] == 0x22`）应在 `callback` 中实现。
    ///
    /// # 示例
    /// ```
    /// let mut capture = Capture::start_with_callback(
    ///     r"\Device\NPF_{GUID}",
    ///     true, 65536, 1000,
    ///     |pkt: &[u8]| {
    ///         // 过滤：仅处理 EtherType == 0x22XX 的包
    ///         if pkt.len() >= 14 && u16::from_be_bytes([pkt[12], pkt[13]]) >> 8 == 0x22 {
    ///             println!("收到自定义协议包，长度: {}", pkt.len());
    ///         }
    ///     },
    /// ).unwrap();
    ///
    /// // ... 等待一段时间后 ...
    /// capture.stop();
    /// ```
    pub fn start_with_callback<F>(
        device_name: &str,
        promisc: bool,
        snaplen: i32,
        timeout_ms: i32,
        callback: F,
    ) -> Result<Self, PcapError>
    where
        F: FnMut(&[u8]) + Send + 'static,
    {
        // 将回调包装为 Box<dyn FnMut>，以便在线程中调用
        let callback = Some(Box::new(callback) as Box<dyn FnMut(&[u8]) + Send>);

        // 创建原子停止标志（调用方和捕获线程共享）
        let running = Arc::new(AtomicBool::new(true));

        let thread_handle = Self::spawn_capture_thread(
            device_name.to_string(),
            promisc,
            snaplen,
            timeout_ms,
            None,
            callback,  // callback 已是 Option<Box<dyn FnMut>>，无需再包 Some
            running.clone(),
        )?;

        let capture = Self {
            running,
            thread_handle: Some(thread_handle),
        };

        Ok(capture)
    }

    // --------------------------------------------------------
    // 公共 API：停止捕获
    // --------------------------------------------------------

    /// 停止捕获循环并等待捕获线程退出。
    ///
    /// 将原子标志 `running` 设为 `false`，捕获线程在下次 `next_packet()` 超时后
    /// 会检测到此标志并退出循环，然后 `join()` 等待线程完全退出。
    ///
    /// 此方法是幂等的：若捕获已停止，调用此方法无副作用。
    ///
    /// 对标 C++ `Capture::end()` 中的 `pcap_breakloop()` + `thread::join()` 逻辑。
    pub fn stop(&mut self) {
        // 将停止标志设为 false，通知捕获线程退出
        self.running.store(false, Ordering::SeqCst);

        // 等待捕获线程退出（若存在）
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// 查询捕获循环是否正在运行。
    ///
    /// 对标 C++ `Capture::isReady()`，但语义不同：
    /// - C++ `isReady()` 检查 `captureHandler_` 是否为 nullptr（句柄是否有效）
    /// - 此函数检查捕获线程是否仍在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    // --------------------------------------------------------
    // 内部方法：启动捕获线程
    // --------------------------------------------------------

    /// 在独立线程中运行 pcap 捕获循环。
    ///
    /// 此方法是通道模式和回调模式的公共实现，
    /// 通过 `tx`（通道发送端）和 `callback`（回调闭包）的 `Option` 组合
    /// 来支持两种消费模式（两者互斥，至多一个为 `Some`）。
    ///
    /// # 线程逻辑
    /// 1. 调用 `Capture::from_device()` 打开网卡（配置混杂模式、snaplen、超时）
    /// 2. 进入 `loop`，每次调用 `next_packet()` 获取数据包
    /// 3. 若 `next_packet()` 返回 `Ok(pkt)`，则通过通道发送或调用回调
    /// 4. 若 `next_packet()` 返回 `TimeoutExpired`（超时），检查 `running` 标志
    ///    - 若 `running` 为 `false`，退出循环
    ///    - 若 `running` 为 `true`，继续等待下一个包
    /// 5. 循环退出后，`Capture` 的 `Drop` 自动关闭 pcap 句柄
    fn spawn_capture_thread(
        device_name: String,
        promisc: bool,
        snaplen: i32,
        timeout_ms: i32,
        tx: Option<mpsc::Sender<Packet>>,
        mut callback: Option<Box<dyn FnMut(&[u8]) + Send>>,
        running: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>, PcapError> {
        // 启动独立线程
        let handle = thread::spawn(move || {
            // ---------- 步骤1：打开网卡 ----------
            // pcap crate 的 builder 模式：
            // 1. Capture::from_device(dev) -> Result<Capture<Inactive>, Error>
            // 2. Inactive.promisc/promisc/snaplen/timeout -> Inactive（配置方法返回 Self）
            // 3. Inactive.open() -> Result<Capture<Active>, Error>
            let device = Device::from(device_name.as_str());

            // 先调用 from_device 获取 Inactive 句柄
            let inactive = match PcapCapture::from_device(device) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("pcap 枚举/打开网卡 '{}' 失败: {}", device_name, e);
                    return;
                }
            };

            // 配置参数（promisc/snaplen/timeout 返回 Inactive，可链式调用）
            let inactive = inactive
                .promisc(promisc)
                .snaplen(snaplen)
                .timeout(timeout_ms);

            // 调用 open() 获取 Active 句柄（开始抓包）
            let mut cap = match inactive.open() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("pcap 激活网卡 '{}' 失败: {}", device_name, e);
                    return;
                }
            };

            // ---------- 步骤2：捕获循环 ----------
            loop {
                // 检查停止标志（在调用 next_packet 之前检查，减少延迟）
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match cap.next_packet() {
                    Ok(pkt) => {
                        // 成功捕获一个数据包
                        let packet = Packet::from_pcap(&pkt);

                        // 根据消费模式处理数据包
                        if let Some(ref tx) = tx {
                            // 通道模式：发送到通道
                            // 若发送失败（接收端已 drop），说明调用方不再需要数据，退出循环
                            if tx.send(packet).is_err() {
                                break;
                            }
                        } else if let Some(ref mut cb) = callback {
                            // 回调模式：调用用户回调
                            cb(&packet.data);
                        }
                    }
                    Err(e) => {
                        // next_packet() 出错
                        match e {
                            pcap::Error::TimeoutExpired => {
                                // 超时：忽略，继续循环（下次循环会检查 running 标志）
                                continue;
                            }
                            _ => {
                                // 其他错误（如设备断开），记录日志后退出循环
                                eprintln!("pcap 捕获错误: {}", e);
                                break;
                            }
                        }
                    }
                }
            }

            // 循环退出，pcap 句柄在此处被 drop，自动调用 pcap_close()
        });

        Ok(handle)
    }
}

// ============================================================
// Drop trait：确保资源自动释放
// ============================================================

impl Drop for Capture {
    /// 当 `Capture` 离开作用域时，自动停止捕获线程并释放资源。
    ///
    /// 对标 C++ `Capture::~Capture()` 中的 `pcap_close()` 调用。
    fn drop(&mut self) {
        self.stop();
    }
}
