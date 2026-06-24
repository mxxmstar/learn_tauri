//! MJPEG（Motion JPEG）流解析与解码模块
//!
//! MJPEG 是一种视频编解码格式，每一帧都是一张完整的 JPEG 图像。
//! 本模块提供：
//! 1. MJPEG 流解析 —— 从字节流中提取独立的 JPEG 帧
//! 2. JPEG 解码 —— 将 JPEG 数据解码为 RGBA 像素数据
//!
//! # MJPEG 流格式
//!
//! ```text
//! [SOI][JPEG 数据][EOI][SOI][JPEG 数据][EOI]...
//! ```
//!
//! - SOI (Start of Image): 0xFF 0xD8
//! - EOI (End of Image):   0xFF 0xD9
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use learn_tauri_lib::render::mjpeg::MjpegParser;
//!
//! let mut parser = MjpegParser::new();
//! let stream_data: &[u8] = &[]; // 从网络或文件读取的 MJPEG 数据
//!
//! // 向解析器喂数据
//! parser.feed(stream_data);
//!
//! // 提取所有完整的帧
//! while let Some(jpeg_data) = parser.next_frame() {
//!     // 解码 JPEG 为 RGBA
//!     match parser.decode_to_rgba(jpeg_data) {
//!         Ok((rgba, width, height)) => {
//!             // 上传到 OpenGL 纹理...
//!         }
//!         Err(e) => eprintln!("解码失败: {}", e),
//!     }
//! }
//! ```

use image::codecs::jpeg::JpegDecoder;
use image::ImageDecoder;
use std::io::Cursor;

use super::error::RenderError;

/// JPEG 标记常量
#[allow(dead_code)]
mod markers {
    /// Start of Image (0xFFD8)
    pub const SOI: u16 = 0xFFD8;
    /// End of Image (0xFFD9)
    pub const EOI: u16 = 0xFFD9;
}

/// MJPEG 流解析器
///
/// 维护内部缓冲区，支持流式输入，从连续的字节流中提取完整的 JPEG 帧。
///
/// ## 工作原理
///
/// 1. 外部数据通过 `feed()` 方法追加到内部缓冲区
/// 2. 每次调用 `next_frame()` 扫描缓冲区查找 SOI/EOI 标记
/// 3. 找到完整帧后返回其数据引用，并移动内部指针
///
/// ## 线程安全
///
/// 本类型未实现 Send/Sync，应在单线程中使用。
/// 如需跨线程传递帧数据，使用 `decode_to_rgba()` 解码后发送 Vec<u8>。
pub struct MjpegParser {
    /// 内部累积缓冲区（未处理的字节数据）
    buffer: Vec<u8>,
    /// 当前解析位置（已处理的数据偏移量）
    offset: usize,
}

impl MjpegParser {
    /// 创建新的 MJPEG 解析器
    pub fn new() -> Self {
        MjpegParser {
            buffer: Vec::with_capacity(1024 * 1024), // 预分配 1MB
            offset: 0,
        }
    }

    /// 向解析器喂入新的流数据
    ///
    /// # 参数
    /// - `data`: 从网络或文件读取的新数据切片
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// 尝试从缓冲区中提取下一个完整的 JPEG 帧
    ///
    /// # 返回值
    /// - `Some(&[u8])`: 找到完整的 JPEG 帧数据（含 SOI 和 EOI 标记）
    /// - `None`: 缓冲区内没有完整的帧（需要更多数据）
    pub fn next_frame(&mut self) -> Option<&[u8]> {
        // 从当前偏移量开始查找 SOI
        let soi_pos = self.find_soi(self.offset)?;

        // 从 SOI 之后查找 EOI
        let eoi_pos = self.find_eoi(soi_pos + 2)?;

        // 提取完整帧（含 SOI 和 EOI）
        let frame_end = eoi_pos + 2;
        let frame = &self.buffer[soi_pos..frame_end];

        // 更新偏移量到帧结束位置，下次从 EOI 之后继续查找
        self.offset = soi_pos + frame.len();

        Some(frame)
    }

    /// 查找 SOI 标记 (0xFF 0xD8)
    fn find_soi(&self, start: usize) -> Option<usize> {
        let data = &self.buffer[start..];
        for i in 0..data.len().saturating_sub(1) {
            if data[i] == 0xFF && data[i + 1] == 0xD8 {
                return Some(start + i);
            }
        }
        None
    }

    /// 查找 EOI 标记 (0xFF 0xD9)
    fn find_eoi(&self, start: usize) -> Option<usize> {
        let data = &self.buffer[start..];
        for i in 0..data.len().saturating_sub(1) {
            if data[i] == 0xFF && data[i + 1] == 0xD9 {
                return Some(start + i);
            }
        }
        None
    }

    /// 将 JPEG 数据解码为 RGBA 像素数据
    ///
    /// 使用 `image` crate 的 JPEG 解码器将 JPEG 数据解码为 RGBA 格式。
    ///
    /// # 参数
    /// - `jpeg_data`: 包含完整 JPEG 帧的字节切片（含 SOI/EOI 标记）
    ///
    /// # 返回值
    /// - `Ok((Vec<u8>, u32, u32))`: (RGBA 像素数据, 宽度, 高度)
    /// - `Err(RenderError)`: 解码失败
    ///
    /// # 性能说明
    /// 每次调用都会进行完整的 JPEG 解码，如果帧率较高建议使用硬件解码。
    pub fn decode_to_rgba(&self, jpeg_data: &[u8]) -> Result<(Vec<u8>, u32, u32), RenderError> {
        // 使用 Cursor 包装数据，使 image 库可以读取
        let cursor = Cursor::new(jpeg_data);

        // 创建 JPEG 解码器
        let decoder = JpegDecoder::new(cursor)
            .map_err(|e: image::ImageError| RenderError::JpegDecode(e.to_string()))?;

        // 获取图像尺寸
        let (width, height) = decoder.dimensions();

        // 分配缓冲区并解码为 RGB
        let total_bytes = decoder.total_bytes() as usize;
        let mut rgb_data = vec![0u8; total_bytes];
        decoder
            .read_image(&mut rgb_data)
            .map_err(|e: image::ImageError| RenderError::JpegDecode(e.to_string()))?;

        // 将 RGB 转换为 RGBA（添加 Alpha 通道）
        let rgba_data = rgb3_to_rgba(&mut rgb_data, (width * height) as usize);

        Ok((rgba_data, width, height))
    }

    /// 重置解析器状态（清空缓冲区和偏移量）
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.offset = 0;
    }

    /// 获取内部缓冲区的数据量
    pub fn buffered_len(&self) -> usize {
        self.buffer.len() - self.offset
    }
}

impl Default for MjpegParser {
    fn default() -> Self {
        Self::new()
    }
}

/// 独立的 JPEG 帧数据
///
/// 表示一帧解码完成后的图像数据，可以跨线程传递。
#[derive(Clone, Debug)]
pub struct DecodedFrame {
    /// RGBA 格式的像素数据
    pub rgba: Vec<u8>,
    /// 图像宽度（像素）
    pub width: u32,
    /// 图像高度（像素）
    pub height: u32,
}

/// 独立的 JPEG 解码函数（不依赖 MjpegParser）
///
/// 如果已有完整的 JPEG 文件数据，可以直接使用此函数解码。
///
/// # 参数
/// - `jpeg_data`: 完整的 JPEG 文件数据（含 SOI/EOI 标记）
///
/// # 返回值
/// 解码成功返回 DecodedFrame，失败返回 RenderError
pub fn decode_jpeg_to_rgba(jpeg_data: &[u8]) -> Result<DecodedFrame, RenderError> {
    let cursor = Cursor::new(jpeg_data);
    let decoder = JpegDecoder::new(cursor)
        .map_err(|e: image::ImageError| RenderError::JpegDecode(e.to_string()))?;

    let (width, height) = decoder.dimensions();

    let total_bytes = decoder.total_bytes() as usize;
    let mut rgb_data = vec![0u8; total_bytes];
    decoder
        .read_image(&mut rgb_data)
        .map_err(|e: image::ImageError| RenderError::JpegDecode(e.to_string()))?;

    let rgba_data = rgb3_to_rgba(&mut rgb_data, (width * height) as usize);

    Ok(DecodedFrame {
        rgba: rgba_data,
        width,
        height,
    })
}

/// 从字节流中查找 JPEG 帧的范围
///
/// 在数据切片中查找第一个完整的 JPEG 帧（从 SOI 到 EOI）。
///
/// # 参数
/// - `data`: 包含 JPEG 数据的字节切片
///
/// # 返回值
/// - `Ok((usize, usize))`: (帧起始位置, 帧长度)，含 SOI/EOI
/// - `Err(RenderError)`: 未找到完整帧
pub fn find_jpeg_frame(data: &[u8]) -> Result<(usize, usize), RenderError> {
    // 查找 SOI
    let soi = data.windows(2).position(|w| w == &[0xFF, 0xD8])
        .ok_or(RenderError::MissingSoi)?;

    // 从 SOI 之后查找 EOI
    let eoi = data[soi + 2..].windows(2).position(|w| w == &[0xFF, 0xD9])
        .ok_or(RenderError::MissingEoi)?;

    let frame_len = eoi + 2 + 2; // SOI + 数据 + EOI

    Ok((soi, frame_len))
}

/// 将 RGB3 格式转换为 RGBA 格式（添加 Alpha 通道，默认不透明）
///
/// # 参数
/// - `rgb_data`: RGB 格式的像素数据（每像素 3 字节）
/// - `pixel_count`: 像素总数
///
/// # 返回值
/// RGBA 格式的像素数据（每像素 4 字节）
fn rgb3_to_rgba(rgb_data: &mut [u8], pixel_count: usize) -> Vec<u8> {
    let src_len = pixel_count * 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);

    // 使用 chunks_exact 高效遍历
    for chunk in rgb_data[..src_len].chunks_exact(3) {
        rgba.push(chunk[0]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[2]); // B
        rgba.push(255);      // A（完全不透明）
    }

    rgba
}
