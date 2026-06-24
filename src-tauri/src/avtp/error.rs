/*!
 * 标准 AVTP 协议解析错误类型
 *
 * 该模块定义了标准 AVTP 协议（IEEE 1722）解析过程中可能遇到的所有错误类型。
 * 使用 `thiserror` 库派生 `Error` trait，提供友好的错误信息。
 *
 * 协议背景：
 * AVTP（Audio Video Transport Protocol，音视频传输协议）是 IEEE 1722 标准定义的
 * 用于在网络上传输音视频数据的链路层协议。它基于以太网，使用 EtherType 0x22F0 标识。
 *
 * AVTP 支持多种格式：
 * - AAF（Audio Applications Format）：纯音频格式
 * - MJPEG（Motion JPEG）：运动 JPEG 视频格式
 * - H.264：H.264 视频格式
 * - etc.
 */

use thiserror::Error;

/// 标准 AVTP 协议解析错误
///
/// 该枚举定义了协议解析过程中可能遇到的所有错误类型。
/// 每个错误变体都包含详细的错误信息，便于调试和日志记录。
#[derive(Error, Debug)]
pub enum AvtpError {
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
    /// 当以太网帧的 EtherType 字段不是 0x22F0 时返回此错误。
    ///
    /// # 参数
    /// - `ethertype`: 实际的 EtherType 值
    #[error("无效的 EtherType: 期望 0x22F0，实际 {0:#06x}")]
    InvalidEtherType(u16),

    /// 无效的 subtype
    ///
    /// 当 AVTP 头部的 subtype 字段不是支持的类型时返回此错误。
    /// 支持的 subtype：
    /// - 0x00：AAF Audio
    /// - 0x07：MJPEG Video
    /// - 0x05：H.264 Video
    ///
    /// # 参数
    /// - `subtype`: 实际的 subtype 值
    #[error("不支持的 AVTP subtype: {0:#04x}")]
    InvalidSubtype(u8),

    /// 版本不匹配
    ///
    /// 当 AVTP 头部的 version 字段不是期望的版本时返回此错误。
    ///
    /// # 参数
    /// - `expected`: 期望的版本号
    /// - `actual`: 实际的版本号
    #[error("AVTP 版本不匹配: 期望 {expected}，实际 {actual}")]
    VersionMismatch {
        expected: u8,
        actual: u8,
    },

    /// 流 ID 不匹配
    ///
    /// 当数据包的流 ID 与期望的流 ID 不匹配时返回此错误。
    /// 用于过滤来自不同设备的流。
    ///
    /// # 参数
    /// - `expected`: 期望的流 ID
    /// - `actual`: 实际的流 ID
    #[error("流 ID 不匹配: 期望 {expected}, 实际 {actual}")]
    StreamIdMismatch {
        expected: u64,
        actual: u64,
    },

    /// 序列号错误
    ///
    /// 当检测到数据包序列号不连续时返回此错误。
    /// 可能表示丢包或乱序。
    ///
    /// # 参数
    /// - `expected`: 期望的序列号
    /// - `actual`: 实际的序列号
    #[error("序列号错误: 期望 {expected}，实际 {actual}")]
    SequenceError {
        expected: u8,
        actual: u8,
    },

    /// 时间戳错误
    ///
    /// 当数据包的时间戳异常时返回此错误。
    /// 可能表示时钟不同步。
    ///
    /// # 参数
    /// - `reason`: 错误原因
    #[error("时间戳错误: {0}")]
    TimestampError(String),

    /// 负载长度错误
    ///
    /// 当头部中的负载长度字段与实际数据长度不匹配时返回此错误。
    ///
    /// # 参数
    /// - `expected`: 期望的负载长度
    /// - `actual`: 实际的负载长度
    #[error("负载长度错误: 期望 {expected} 字节，实际 {actual} 字节")]
    PayloadLengthError {
        expected: u16,
        actual: usize,
    },

    /// 解析错误
    ///
    /// 通用的解析错误，用于描述其他无法归类的问题。
    ///
    /// # 参数
    /// - `msg`: 错误描述
    #[error("解析错误: {0}")]
    ParseError(String),

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
/// 为标准 AVTP 协议解析模块提供统一的结果类型。
pub type Result<T> = std::result::Result<T, AvtpError>;
