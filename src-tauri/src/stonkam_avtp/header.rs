/*!
 * Stonkam 自定义 AVTP 协议头部定义
 *
 * 该模块定义了 Stonkam 自定义协议（EtherType 0x0022）的头部格式和解析逻辑。
 *
 * 协议格式（基于逆向工程分析）：
 * ```
 * 以太网帧（14 字节）
 * ├── 目标 MAC (6 字节)
 * ├── 源 MAC (6 字节)
 * └── EtherType = 0x0022 (2 字节)
 *
 * 自定义协议头部（24 字节，以太网帧偏移 14-37）
 * ├── 保留/版本 (1 字节)                   [以太网帧偏移量 14]
 * ├── 帧起始标志 (1 字节)                  [以太网帧偏移量 15]
 * ├── 保留字段 (4 字节)                    [以太网帧偏移量 16-19]
 * ├── 保留字段 (14 字节)                   [以太网帧偏移量 20-33]
 * ├── 负载长度 (2 字节，大端)             [以太网帧偏移量 34-35]
 * ├── 帧结束标志 (1 字节)                  [以太网帧偏移量 36]
 * ├── 保留字段 (1 字节)                    [以太网帧偏移量 37]
 * │
 * └── JPEG 数据 (N 字节)                   [以太网帧偏移量 38+]
 *     ├── 嵌入式头部 (12 字节)             [JPEG 数据偏移量 0-11]
 *     │   ├── 未知 (5 字节)
 *     │   ├── 质量因子 QP (1 字节)
 *     │   ├── 图像宽度 (1 字节，×8)
 *     │   ├── 图像高度 (1 字节，×8)
 *     │   ├── 重启间隔 (2 字节)
 *     │   └── 帧计数 (2 字节)
 *     │
 *     └── JPEG 字节流 (N-12 字节)         [JPEG 数据偏移量 12+]
 * ```
 */

use crate::stonkam_avtp::error::{Result, StonkamAvtpError};

/// Stonkam 自定义 AVTP 协议头部
///
/// 该结构体表示 Stonkam 自定义协议的 24 字节头部。
/// 头部位于以太网帧偏移 14-37 的位置。
///
/// # 字段说明
/// - `frame_start`: 帧起始标志（bit 0 = 1 表示起始包）
/// - `frame_end`: 帧结束标志（bit 4 = 1 表示结束包）
/// - `payload_len`: JPEG 数据长度（字节）
/// - `reserved`: 保留字段（用途未知）
#[derive(Debug, Clone)]
pub struct StonkamAvtpHeader {
    /// 以太网帧原始数据（用于调试）
    #[doc(hidden)]
    pub raw_ethernet_frame: Vec<u8>,

    /// 帧起始标志
    ///
    /// 位于以太网帧偏移量 15（协议头部偏移量 1）。
    /// bit 0 = 1 表示该包是 JPEG 帧的起始包。
    pub frame_start: bool,

    /// 帧结束标志
    ///
    /// 位于以太网帧偏移量 36（协议头部偏移量 22）。
    /// bit 4 = 1 表示该包是 JPEG 帧的结束包。
    pub frame_end: bool,

    /// 负载长度
    ///
    /// 位于以太网帧偏移量 34-35（协议头部偏移量 20-21）。
    /// 表示 JPEG 数据的长度（大端字节序）。
    pub payload_len: u16,

    /// 保留字段（用途未知）
    ///
    /// 协议头部中有多个保留字段，目前未知其具体用途。
    /// 可能包含：时间戳、序列号、设备 ID 等。
    pub reserved: [u8; 20],
}

impl StonkamAvtpHeader {
    /// 从以太网帧数据解析协议头部
    ///
    /// 该方法从原始以太网帧数据中提取 Stonkam 自定义协议的头部信息。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据（至少 38 字节）
    ///
    /// # 返回值
    /// - `Ok(StonkamAvtpHeader)`: 解析成功
    /// - `Err(StonkamAvtpError)`: 解析失败
    ///
    /// # 错误
    /// - `BufferTooShort`: 缓冲区长度不足
    /// - `InvalidEtherType`: 不是 Stonkam 协议（EtherType != 0x0022）
    ///
    /// # 示例
    /// ```rust
    /// use crate::stonkam_avtp::header::StonkamAvtpHeader;
    ///
    /// let frame = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55,  // 目标 MAC
    ///                 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB,  // 源 MAC
    ///                 0x00, 0x22,  // EtherType = 0x0022
    ///                 // ... 协议数据
    ///                ];
    /// let header = StonkamAvtpHeader::from_ethernet_frame(&frame)?;
    /// ```
    pub fn from_ethernet_frame(ethernet_frame: &[u8]) -> Result<Self> {
        // 检查缓冲区长度
        // 至少需要：以太网头（14 字节）+ 协议头部（24 字节）= 38 字节
        if ethernet_frame.len() < 38 {
            return Err(StonkamAvtpError::BufferTooShort {
                need: 38,
                got: ethernet_frame.len(),
            });
        }

        // 检查 EtherType 是否为 0x0022
        // 注意：代码中只检查 packet[12] == 0x22（低字节），这里我们也保持一致
        // 正确的检查应该是：((packet[12] << 8) | packet[13]) == 0x0022
        let ethertype_low = ethernet_frame[12];
        let ethertype_high = ethernet_frame[13];

        // 兼容原有代码的逻辑：只检查低字节
        if ethertype_low != 0x22 {
            return Err(StonkamAvtpError::InvalidEtherType(
                ((ethertype_high as u16) << 8) | (ethertype_low as u16),
            ));
        }

        // 解析帧起始标志（以太网帧偏移量 15）
        // bit 0 = 1 表示起始包
        let frame_start = (ethernet_frame[15] & 0x01) != 0;

        // 解析帧结束标志（以太网帧偏移量 36）
        // bit 4 = 1 表示结束包
        let frame_end = (ethernet_frame[36] & 0x10) != 0;

        // 解析负载长度（以太网帧偏移量 34-35，大端字节序）
        let payload_len = ((ethernet_frame[34] as u16) << 8) | (ethernet_frame[35] as u16);

        // 提取保留字段（以太网帧偏移量 14-15, 16-33, 37）
        let mut reserved = [0u8; 20];
        reserved[0] = ethernet_frame[14];  // 版本/保留
        reserved[1] = ethernet_frame[15];  // 帧标志（含起始标志）
        reserved[2..6].copy_from_slice(&ethernet_frame[16..20]);  // 保留字段 1
        reserved[6..20].copy_from_slice(&ethernet_frame[20..34]);  // 保留字段 2
        // 注意：reserved[1] 已经包含了帧标志，这里重复存储是为了完整性

        Ok(Self {
            raw_ethernet_frame: ethernet_frame.to_vec(),
            frame_start,
            frame_end,
            payload_len,
            reserved,
        })
    }

    /// 获取 JPEG 数据起始位置
    ///
    /// JPEG 数据位于以太网帧偏移量 38 的位置。
    ///
    /// # 返回值
    /// - JPEG 数据在以太网帧中的起始偏移量（固定为 38）
    pub fn jpeg_data_offset() -> usize {
        38
    }

    /// 获取 JPEG 数据
    ///
    /// 从以太网帧中提取 JPEG 数据部分（偏移量 38 开始）。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    ///
    /// # 返回值
    /// - `Ok(&[u8])`: JPEG 数据切片
    /// - `Err(StonkamAvtpError)`: 数据长度不足
    pub fn get_jpeg_data<'a>(ethernet_frame: &'a [u8]) -> Result<&'a [u8]> {
        let offset = Self::jpeg_data_offset();
        if ethernet_frame.len() <= offset {
            return Err(StonkamAvtpError::BufferTooShort {
                need: offset + 1,
                got: ethernet_frame.len(),
            });
        }

        Ok(&ethernet_frame[offset..])
    }

    /// 解析 JPEG 嵌入式头部
    ///
    /// JPEG 数据的前 12 字节是嵌入式头部，包含图像参数。
    ///
    /// # 参数
    /// - `jpeg_data`: JPEG 数据（至少 12 字节）
    ///
    /// # 返回值
    /// - `Ok(JpegEmbeddedHeader)`: 解析成功
    /// - `Err(StonkamAvtpError)`: 数据长度不足
    pub fn parse_jpeg_embedded_header(jpeg_data: &[u8]) -> Result<JpegEmbeddedHeader> {
        if jpeg_data.len() < 12 {
            return Err(StonkamAvtpError::JpegDataTooShort(jpeg_data.len()));
        }

        let qp = jpeg_data[5];                          // 质量因子
        let width = (jpeg_data[6] as u16) * 8;       // 图像宽度（像素）
        let height = (jpeg_data[7] as u16) * 8;      // 图像高度（像素）
        let rst_int = ((jpeg_data[8] as u16) << 8) | (jpeg_data[9] as u16);  // 重启间隔
        let rst_count = (((jpeg_data[10] as u16) << 8) | (jpeg_data[11] as u16)) & 0x3FF;  // 帧计数

        Ok(JpegEmbeddedHeader {
            qp,
            width,
            height,
            rst_int,
            rst_count,
            raw: Vec::from(&jpeg_data[0..12]),
        })
    }
}

/// JPEG 嵌入式头部
///
/// 该结构体表示嵌入在 JPEG 数据流前 12 字节中的图像参数。
///
/// # 字段说明
/// - `qp`: JPEG 质量因子（1-100）
/// - `width`: 图像宽度（像素）
/// - `height`: 图像高度（像素）
/// - `rst_int`: JPEG 重启标记间隔
/// - `rst_count`: 帧计数器（低 10 位有效）
/// - `raw`: 原始 12 字节数据
#[derive(Debug, Clone)]
pub struct JpegEmbeddedHeader {
    /// JPEG 质量因子
    ///
    /// 位于 JPEG 数据偏移量 5。
    /// 取值范围：1-100，值越大质量越高、文件越大。
    pub qp: u8,

    /// 图像宽度（像素）
    ///
    /// 位于 JPEG 数据偏移量 6。
    /// 实际宽度 = 值 × 8 像素。
    pub width: u16,

    /// 图像高度（像素）
    ///
    /// 位于 JPEG 数据偏移量 7。
    /// 实际高度 = 值 × 8 像素。
    pub height: u16,

    /// JPEG 重启标记间隔
    ///
    /// 位于 JPEG 数据偏移量 8-9（大端字节序）。
    /// 表示每隔多少个 MCU（Minimum Coding Unit）插入一个 RST 标记。
    pub rst_int: u16,

    /// 帧计数器
    ///
    /// 位于 JPEG 数据偏移量 10-11（大端字节序）。
    /// 只有低 10 位有效（& 0x3FF）。
    /// 用途：可能是帧编号，用于检测丢帧。
    pub rst_count: u16,

    /// 原始 12 字节数据
    #[doc(hidden)]
    pub raw: Vec<u8>,
}

impl JpegEmbeddedHeader {
    /// 获取 JPEG 熵编码数据起始位置
    ///
    /// JPEG 熵编码数据位于嵌入式头部偏移量 12 的位置。
    ///
    /// # 返回值
    /// - 熵编码数据在 JPEG 数据中的起始偏移量（固定为 12）
    pub fn entropy_data_offset() -> usize {
        12
    }
}
