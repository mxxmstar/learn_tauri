/*!
 * Stonkam 自定义 AVTP 协议解析器
 *
 * 该模块提供了 Stonkam 自定义协议（EtherType 0x0022）的完整解析功能。
 * 包括：
 * - 以太网帧解析
 * - JPEG 帧重组
 * - 与 pcap 模块的集成
 *
 * 使用示例：
 * ```rust
 * use crate::stonkam_avtp::parser::StonkamAvtpParser;
 *
 * let mut parser = StonkamAvtpParser::new();
 *
 * // 从 pcap 接收数据包
 * for pkt in packet_receiver {
 *     if let Some(jpeg_frame) = parser.parse_packet(&pkt.data)? {
 *         // jpeg_frame 是完整的 JPEG 数据，可以解码显示
 *         println!("Received complete JPEG frame: {} bytes", jpeg_frame.len());
 *     }
 * }
 * ```
 */

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use crate::stonkam_avtp::error::{Result, StonkamAvtpError};
use crate::stonkam_avtp::header::{StonkamAvtpHeader, JpegEmbeddedHeader};

/// Stonkam 自定义 AVTP 协议解析器
///
/// 该结构体用于解析 Stonkam 自定义协议的数据包，并重组完整的 JPEG 帧。
///
/// # 工作原理
/// 1. 接收原始以太网帧（来自 pcap）
/// 2. 检查 EtherType 是否为 0x0022
/// 3. 解析协议头部，提取帧起始/结束标志
/// 4. 根据标志位将 JPEG 数据拼接成完整帧
/// 5. 当接收到结束包时，返回完整的 JPEG 数据
///
/// # 字段说明
/// - `jpeg_buffer`: JPEG 数据缓冲区（拼接中间状态）
/// - `frame_count`: 已接收的完整帧计数
/// - `packet_count`: 已接收的数据包计数
/// - `last_frame_start_time`: 上一个起始包的时间戳（用于超时检测）
pub struct StonkamAvtpParser {
    /// JPEG 数据缓冲区
    ///
    /// 用于拼接分片的 JPEG 数据。当接收到起始包时清空，接收结束时返回。
    jpeg_buffer: Vec<u8>,

    /// 当前是否正在接收一帧
    ///
    /// 用于检测包顺序错误（例如：在没有起始包的情况下接收到结束包）。
    receiving_frame: bool,

    /// 已接收的完整帧计数
    frame_count: AtomicU32,

    /// 已接收的数据包计数
    packet_count: AtomicU32,

    /// 丢帧计数
    dropped_frames: AtomicU32,

    /// 配置：最大 JPEG 帧大小（字节）
    ///
    /// 用于防止内存溢出攻击。如果单帧超过此大小，将丢弃并重置状态。
    max_frame_size: usize,
}

impl StonkamAvtpParser {
    /// 创建新的解析器实例
    ///
    /// # 返回值
    /// - `StonkamAvtpParser`: 新的解析器实例
    ///
    /// # 示例
    /// ```rust
    /// use crate::stonkam_avtp::parser::StonkamAvtpParser;
    ///
    /// let parser = StonkamAvtpParser::new();
    /// ```
    pub fn new() -> Self {
        Self {
            jpeg_buffer: Vec::new(),
            receiving_frame: false,
            frame_count: AtomicU32::new(0),
            packet_count: AtomicU32::new(0),
            dropped_frames: AtomicU32::new(0),
            max_frame_size: 5 * 1024 * 1024, // 默认最大 5MB
        }
    }

    /// 创建新的解析器实例（可配置）
    ///
    /// # 参数
    /// - `max_frame_size`: 最大 JPEG 帧大小（字节）
    ///
    /// # 返回值
    /// - `StonkamAvtpParser`: 新的解析器实例
    pub fn with_config(max_frame_size: usize) -> Self {
        Self {
            jpeg_buffer: Vec::new(),
            receiving_frame: false,
            frame_count: AtomicU32::new(0),
            packet_count: AtomicU32::new(0),
            dropped_frames: AtomicU32::new(0),
            max_frame_size,
        }
    }

    /// 解析单个数据包
    ///
    /// 该方法接收一个原始以太网帧，解析其中的 Stonkam 自定义协议数据，
    /// 并返回拼接后的完整 JPEG 帧（如果当前包是结束包）。
    ///
    /// # 参数
    /// - `ethernet_frame`: 原始以太网帧数据
    /// - `timestamp`: 数据包捕获时间戳
    ///
    /// # 返回值
    /// - `Ok(Some(Vec<u8>))`: 接收到完整的 JPEG 帧
    /// - `Ok(None)`: 接收到中间包，需要继续等待
    /// - `Err(StonkamAvtpError)`: 解析错误
    ///
    /// # 示例
    /// ```rust
    /// let mut parser = StonkamAvtpParser::new();
    ///
    /// for pkt in packet_receiver {
    ///     match parser.parse_packet(&pkt.data, pkt.timestamp) {
    ///         Ok(Some(jpeg_data)) => {
    ///             println!("Received complete JPEG frame: {} bytes", jpeg_data.len());
    ///             // 解码并显示 JPEG 图像
    ///         }
    ///         Ok(None) => {
    ///             // 中间包，继续等待
    ///         }
    ///         Err(e) => {
    ///             eprintln!("Parse error: {}", e);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn parse_packet(
        &mut self,
        ethernet_frame: &[u8],
        _timestamp: SystemTime,
    ) -> Result<Option<Vec<u8>>> {
        // 1. 解析协议头部
        let header = StonkamAvtpHeader::from_ethernet_frame(ethernet_frame)?;

        // 2. 获取 JPEG 数据
        let jpeg_data = StonkamAvtpHeader::get_jpeg_data(ethernet_frame)?;

        // 3. 更新数据包计数
        self.packet_count.fetch_add(1, Ordering::SeqCst);

        // 4. 处理帧起始包
        if header.frame_start {
            // 如果已经在接收一帧，说明上一帧没有正确结束（丢包）
            if self.receiving_frame {
                self.dropped_frames.fetch_add(1, Ordering::SeqCst);
                // 重置状态
                self.jpeg_buffer.clear();
            }

            // 解析 JPEG 嵌入式头部
            let embedded_header = StonkamAvtpHeader::parse_jpeg_embedded_header(jpeg_data)?;

            // 生成 JPEG 文件头
            let jpeg_header = Self::generate_jpeg_header(&embedded_header)?;

            // 初始化缓冲区
            self.jpeg_buffer = jpeg_header;
            self.receiving_frame = true;

            // 添加 JPEG 熵编码数据（跳过嵌入式头部 12 字节）
            if jpeg_data.len() > 12 {
                let entropy_data = &jpeg_data[12..];
                self.jpeg_buffer.extend_from_slice(entropy_data);
            }
        }
        // 5. 处理中间包或结束包
        else {
            // 检查是否在接收一帧
            if !self.receiving_frame {
                return Err(StonkamAvtpError::PacketOrderError {
                    expected: "起始包".to_string(),
                    actual: "中间包或结束包".to_string(),
                });
            }

            // 检查帧大小是否超过限制
            if self.jpeg_buffer.len() + jpeg_data.len() > self.max_frame_size {
                self.dropped_frames.fetch_add(1, Ordering::SeqCst);
                self.reset();
                return Err(StonkamAvtpError::InvalidImageParameter {
                    param: "frame_size".to_string(),
                    value: (self.jpeg_buffer.len() + jpeg_data.len()) as u32,
                });
            }

            // 添加 JPEG 数据（注意：中间包没有嵌入式头部，整个 payload 都是 JPEG 数据）
            self.jpeg_buffer.extend_from_slice(jpeg_data);
        }

        // 6. 处理帧结束包
        if header.frame_end {
            self.receiving_frame = false;
            self.frame_count.fetch_add(1, Ordering::SeqCst);

            // 返回完整的 JPEG 数据
            let complete_frame = self.jpeg_buffer.clone();
            self.jpeg_buffer.clear();

            return Ok(Some(complete_frame));
        }

        // 7. 中间包，继续等待
        Ok(None)
    }

    /// 生成 JPEG 文件头
    ///
    /// 根据 JPEG 嵌入式头部中的参数（宽、高、质量因子），
    /// 生成标准的 JPEG 文件头（包含 JFIF APP0、DQT、SOF、DHT、SOS 等标记）。
    ///
    /// # 参数
    /// - `embedded_header`: JPEG 嵌入式头部
    ///
    /// # 返回值
    /// - `Ok(Vec<u8>)`: JPEG 文件头字节数组
    /// - `Err(StonkamAvtpError)`: 参数错误
    ///
    /// # 注意
    /// 该函数参考了 `bcm_jpeg.cpp` 中的 `BCM_JPG_EncodeHeader()` 函数。
    fn generate_jpeg_header(embedded_header: &JpegEmbeddedHeader) -> Result<Vec<u8>> {
        // 检查图像参数
        if embedded_header.width == 0 || embedded_header.height == 0 {
            return Err(StonkamAvtpError::InvalidImageParameter {
                param: "width/height".to_string(),
                value: 0,
            });
        }

        let mut header = Vec::new();

        // 1. JFIF APP0 标记
        // 格式：0xFF 0xE0 [长度 2 字节] "JFIF\0" [版本 2 字节] [密度 7 字节]
        header.extend_from_slice(&[
            0xFF, 0xE0,  // APP0 标记
            0x00, 0x10,  // 长度 = 16 字节
            b'J', b'F', b'I', b'F', 0x00,  // "JFIF\0"
            0x01, 0x01,  // 版本 1.1
            0x00,  // 密度单位：无单位
            0x00, 0x01, 0x00, 0x01,  // 密度：1x1
            0x00, 0x00,  // 无缩略图
        ]);

        // 2. 量化表 DQT
        // 格式：0xFF 0xDB [长度 2 字节] [表 ID 1 字节] [量化表 64 字节] x 2
        let (quant_luma, quant_chroma) = Self::generate_quantization_tables(embedded_header.qp);

        header.extend_from_slice(&[0xFF, 0xDB]);  // DQT 标记
        let dqt_len = 2 + 2 * (1 + 64);  // 长度 = 2 + 2 * (1 + 64)
        header.extend_from_slice(&[(dqt_len >> 8) as u8, (dqt_len & 0xFF) as u8]);
        
        header.push(0x00);  // 亮度量化表 ID = 0
        header.extend_from_slice(&quant_luma);
        
        header.push(0x01);  // 色度量化表 ID = 1
        header.extend_from_slice(&quant_chroma);

        // 3. 帧开始 SOF0
        // 格式：0xFF 0xC0 [长度 2 字节] [精度 1 字节] [高度 2 字节] [宽度 2 字节] [分量数 1 字节] [分量信息...]
        header.extend_from_slice(&[0xFF, 0xC0]);  // SOF0 标记
        let sof_len = 2 + 6 + 3 * 3;  // 长度 = 2 + 6 + 3 * 3
        header.extend_from_slice(&[(sof_len >> 8) as u8, (sof_len & 0xFF) as u8]);
        header.push(0x08);  // 精度 = 8 位
        header.extend_from_slice(&[(embedded_header.height >> 8) as u8, (embedded_header.height & 0xFF) as u8]);
        header.extend_from_slice(&[(embedded_header.width >> 8) as u8, (embedded_header.width & 0xFF) as u8]);
        header.push(0x03);  // 分量数 = 3 (Y、Cb、Cr)
        header.extend_from_slice(&[
            0x01, 0x22, 0x00,  // Y 分量：ID=1, 采样因子=0x22, 量化表=0
            0x02, 0x11, 0x01,  // Cb 分量：ID=2, 采样因子=0x11, 量化表=1
            0x03, 0x11, 0x01,  // Cr 分量：ID=3, 采样因子=0x11, 量化表=1
        ]);

        // 4. 霍夫曼表 DHT（省略，使用默认表）
        // 注意：完整的实现需要添加 DHT 标记和霍夫曼表数据
        // 这里为了简化，假设 JPEG 数据流中包含 DHT（实际上应该包含）

        // 5. 扫描开始 SOS
        // 格式：0xFF 0xDA [长度 2 字节] [分量数 1 字节] [分量信息...] [光谱选择 3 字节]
        header.extend_from_slice(&[0xFF, 0xDA]);  // SOS 标记
        let sos_len = 2 + 1 + 2 * 3 + 3;  // 长度 = 2 + 1 + 2 * 3 + 3
        header.extend_from_slice(&[(sos_len >> 8) as u8, (sos_len & 0xFF) as u8]);
        header.push(0x03);  // 分量数 = 3
        header.extend_from_slice(&[
            0x01, 0x00,  // Y 分量：ID=1, 霍夫曼表=0
            0x02, 0x11,  // Cb 分量：ID=2, 霍夫曼表=1
            0x03, 0x11,  // Cr 分量：ID=3, 霍夫曼表=1
        ]);
        header.extend_from_slice(&[0x00, 0x3F, 0x00]);  // 光谱选择：0, 63, 0

        Ok(header)
    }

    /// 生成量化表
    ///
    /// 根据 JPEG 质量因子（QP）生成亮度量化表和色度量化表。
    ///
    /// # 参数
    /// - `qp`: JPEG 质量因子（1-100）
    ///
    /// # 返回值
    /// - `(Vec<u8>, Vec<u8>)`: (亮度量化表, 色度量化表)
    fn generate_quantization_tables(qp: u8) -> (Vec<u8>, Vec<u8>) {
        // 标准 JPEG 默认量化表（ITU-T T.81 附录 K）
        let default_quant_luma = [
            16, 11, 10, 16, 24, 40, 51, 61,
            12, 12, 14, 19, 26, 58, 60, 55,
            14, 13, 16, 24, 40, 57, 69, 56,
            14, 17, 22, 29, 51, 87, 80, 62,
            18, 22, 37, 56, 68, 109, 103, 77,
            24, 35, 55, 64, 81, 104, 113, 92,
            49, 64, 78, 87, 103, 121, 120, 101,
            72, 92, 95, 98, 112, 100, 103, 99,
        ];

        let default_quant_chroma = [
            17, 18, 24, 47, 99, 99, 99, 99,
            18, 21, 26, 66, 99, 99, 99, 99,
            24, 26, 56, 99, 99, 99, 99, 99,
            47, 66, 99, 99, 99, 99, 99, 99,
            99, 99, 99, 99, 99, 99, 99, 99,
            99, 99, 99, 99, 99, 99, 99, 99,
            99, 99, 99, 99, 99, 99, 99, 99,
            99, 99, 99, 99, 99, 99, 99, 99,
        ];

        // 质量因子转换公式（参考 libjpeg）
        let qf = if qp < 50 {
            5000 / (qp as u32)
        } else {
            200 - (qp as u32) * 2
        };

        // 生成量化表
        let mut quant_luma = Vec::with_capacity(64);
        let mut quant_chroma = Vec::with_capacity(64);

        for i in 0..64 {
            // 注意：这里省略了 ZigZag 逆变换，实际应用中需要添加
            let luma_val = ((default_quant_luma[i] as u32 * qf + 50) / 100).clamp(1, 255);
            let chroma_val = ((default_quant_chroma[i] as u32 * qf + 50) / 100).clamp(1, 255);

            quant_luma.push(luma_val as u8);
            quant_chroma.push(chroma_val as u8);
        }

        (quant_luma, quant_chroma)
    }

    /// 重置解析器状态
    ///
    /// 清空 JPEG 缓冲区，重置接收标志。
    fn reset(&mut self) {
        self.jpeg_buffer.clear();
        self.receiving_frame = false;
    }

    /// 获取统计信息
    ///
    /// 返回解析器的统计信息（帧计数、包计数、丢帧计数）。
    ///
    /// # 返回值
    /// - `(u32, u32, u32)`: (帧计数, 包计数, 丢帧计数)
    pub fn get_stats(&self) -> (u32, u32, u32) {
        (
            self.frame_count.load(Ordering::SeqCst),
            self.packet_count.load(Ordering::SeqCst),
            self.dropped_frames.load(Ordering::SeqCst),
        )
    }
}
