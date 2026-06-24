//! 网卡设备枚举与信息结构体
//!
//! 封装 `pcap::Device::list()`，提供对网卡设备的友好抽象。
//! 对标 C++ 中 `pcap_findalldevs()` / `pcap_if_t` 的功能。

use crate::pcap::error::PcapError;
use pcap::Device;

/// 表示一个可用的网卡设备。
///
/// 对标 C++ `pcap_if_t` 结构体，包含设备名称（用于打开）和
/// 人类可读的描述信息。
#[derive(Debug, Clone)]
pub struct NetworkDevice {
    /// 网卡设备名称（如 `"\\Device\\NPF_{GUID}"` 或 `"eth0"`）。
    ///
    /// 将此名称传给 `Capture::open()` 可打开该网卡进行抓包。
    pub name: String,

    /// 网卡描述信息（如 `"Intel(R) Ethernet Connection"`）。
    ///
    /// 在 Windows 上通常来自 Npcap；在 Linux 上可能为空。
    pub description: Option<String>,
}

/// 枚举当前系统中所有可用的网卡设备。
///
/// 对标 C++ `Capture::getNetworkInterface()` 方法，
/// 内部调用 `pcap::Device::list()` 获取设备列表。
///
/// # 返回值
/// 成功时返回 `NetworkDevice` 列表，每个元素包含设备名称和描述。
///
/// # 错误
/// - 若底层 `pcap` 库调用失败（如 Npcap 未安装、权限不足），
///   返回 `PcapError::ListDevicesError`。
///
/// # 示例
/// ```
/// use crate::pcap::device::list_devices;
///
/// let devices = list_devices().unwrap();
/// for dev in &devices {
///     println!("网卡: {} - {}", dev.name, dev.description.as_deref().unwrap_or(""));
/// }
/// ```
pub fn list_devices() -> Result<Vec<NetworkDevice>, PcapError> {
    // 调用 pcap crate 的 Device::list() 枚举所有网卡
    let devices = Device::list()
        .map_err(|e| PcapError::ListDevicesError(e.to_string()))?;

    // 将 pcap::Device 转换为自定义的 NetworkDevice 结构体
    let result: Vec<NetworkDevice> = devices
        .into_iter()
        .map(|dev| NetworkDevice {
            name: dev.name,
            description: dev.desc,
        })
        .collect();

    Ok(result)
}
