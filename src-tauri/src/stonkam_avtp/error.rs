/*!
 * Stonkam 自定义 AVTP 协议解析错误类型
 *
 * 该模块定义了 Stonkam 自定义协议（EtherType 0x0022）解析过程中可能遇到的所有错误类型。
 * 使用 `thiserror` 库派生 `Error` trait，提供友好的错误信息。
 *
 * 协议背景：
 * Stonkam 是一家车载视频监控设备制造商，其设备使用自定义的以太网协议传输视频流。
 * 该协议基于以太网层（Layer 2），使用 EtherType 0x0022 标识，直接传输 JPEG 图像数据。
 */

use thiserror::Error;

/// Stonkam 自定义 AVTP 协议解析错误
///
/// 该枚举定义了协议解析过程中可能遇到的所有错误类型。
/// 每个错误变体都包含详细的错误信息，便于调试和日志记录。
#[derive(Error, Debug)]
pub enum StonkamAvtpError {
    /// 缓冲区长度不足
    ///
    /// 当提供的字节数组长度小于协议头部所需的最小长度时返回此错误。
    ///
    /// # 参数
    /// - `need`: 需要的字节数
    /// - `got`: 实际提供的字节数
    #[error("缓冲区长度不足: 需要 {need} 字节，实际 {got} 字节")]
    BufferTooShort {
        need: usize,
        got: usize,
    },

    /// 无效的 EtherType
    ///
    /// 当以太网帧的 EtherType 字段不是 0x0022 时返回此错误。
    ///
    /// # 参数
    /// - `ethertype`: 实际的 EtherType 值
    #[error("无效的 EtherType: 期望 0x0022，实际 {0:#06x}")]
    InvalidEtherType(u16),

    /// 无效的帧标志组合
    ///
    /// 当帧起始标志和帧结束标志的组合无效时返回此错误。
    /// 例如：两者都为 0（表示中间包，但没有起始包）
    ///
    /// # 参数
    /// - `start`: 帧起始标志
    /// - `end`: 帧结束标志
    #[error("无效的帧标志组合: start={start}, end={end}")]
    InvalidFrameFlags {
        start: u8,
        end: u8,
    },

    /// JPEG 数据长度不足
    ///
    /// 当 JPEG 数据部分的长度小于嵌入式头部（12 字节）时返回此错误。
    #[error("JPEG 数据长度不足: 需要至少 12 字节的嵌入式头部，实际 {0} 字节")]
    JpegDataTooShort(usize),

    /// JPEG 解码失败
    ///
    /// 当 JPEG 数据无法被正常解码时返回此错误。
    /// 可能的原因：数据损坏、参数错误、不支持的 JPEG 格式等。
    ///
    /// # 参数
    /// - `reason`: 失败原因
    #[error("JPEG 解码失败: {0}")]
    JpegDecodeError(String),

    /// 包顺序错误
    ///
    /// 当接收到的数据包顺序不正确时返回此错误。
    /// 例如：在接收到起始包之前就接收到了中间包或结束包。
    #[error("包顺序错误: 期望 {expected}，实际 {actual}")]
    PacketOrderError {
        expected: String,
        actual: String,
    },

    /// 图像参数错误
    ///
    /// 当 JPEG 嵌入式头部中的图像参数无效时返回此错误。
    /// 例如：宽度为 0、高度超过限制等。
    ///
    /// # 参数
    /// - `param`: 参数名称
    /// - `value`: 参数值
    #[error("图像参数错误: {param} = {value}")]
    InvalidImageParameter {
        param: String,
        value: u32,
    },

    /// pcap 相关错误
    ///
    /// 当与 pcap 库交互发生错误时返回此错误。
    ///
    /// # 参数
    /// - `msg`: 错误描述
    #[error("pcap 错误: {0}")]
    PcapError(String),
}

/// 结果类型别名
///
/// 为 Stonkam 自定义 AVTP 协议解析模块提供统一的结果类型。
pub type Result<T> = std::result::Result<T, StonkamAvtpError>;
