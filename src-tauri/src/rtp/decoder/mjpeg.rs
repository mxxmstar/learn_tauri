//! MJPEG 解码器
//!
//! 使用 image crate 实现 MJPEG 解码

use image::{DynamicImage, ImageFormat};
use crate::rtp::decoder::frame::{MediaFrame, MediaPacket};
use crate::rtp::decoder::trait_::{DecodeError, DecodeResult, Decoder};
use crate::rtp::decoder::types::{CodecType, PixelFormat};

/// MJPEG 解码器
///
/// 使用 image crate 解码 MJPEG 帧
pub struct MjpegDecoder {
    /// 输出像素格式
    output_format: PixelFormat,
    /// 最后一次解码的图像尺寸
    last_width: u32,
    last_height: u32,
}

impl MjpegDecoder {
    /// 创建新的 MJPEG 解码器
    ///
    /// 默认输出 RGBA 格式
    pub fn new() -> Self {
        Self {
            output_format: PixelFormat::RGBA,
            last_width: 0,
            last_height: 0,
        }
    }

    /// 创建指定输出格式的 MJPEG 解码器
    pub fn with_output_format(output_format: PixelFormat) -> Self {
        Self {
            output_format,
            last_width: 0,
            last_height: 0,
        }
    }

    /// 设置输出像素格式
    pub fn set_output_format(&mut self, format: PixelFormat) {
        self.output_format = format;
    }

    /// 获取输出像素格式
    pub fn output_format(&self) -> PixelFormat {
        self.output_format
    }

    /// 解码 JPEG 数据为 DynamicImage
    fn decode_jpeg(&self, data: &[u8]) -> DecodeResult<DynamicImage> {
        image::load_from_memory(data)
            .map_err(|e| DecodeError::DecodeFailed(format!("JPEG decode failed: {}", e)))
    }

    /// 将 DynamicImage 转换为指定的像素格式
    fn convert_format(&self, img: DynamicImage) -> DecodeResult<(Vec<u8>, u32, u32, PixelFormat)> {
        let width = img.width();
        let height = img.height();

        match self.output_format {
            PixelFormat::RGBA => {
                let rgba = img.into_rgba8();
                Ok((rgba.into_raw(), width, height, PixelFormat::RGBA))
            }
            PixelFormat::RGB => {
                let rgb = img.into_rgb8();
                Ok((rgb.into_raw(), width, height, PixelFormat::RGB))
            }
            PixelFormat::BGRA => {
                // image crate 不直接支持 BGRA，需要手动转换
                let rgba = img.into_rgba8();
                let mut bgra = Vec::with_capacity(rgba.len());
                for chunk in rgba.chunks(4) {
                    bgra.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
                }
                Ok((bgra, width, height, PixelFormat::BGRA))
            }
            PixelFormat::BGR => {
                // image crate 不直接支持 BGR，需要手动转换
                let rgb = img.into_rgb8();
                let mut bgr = Vec::with_capacity(rgb.len());
                for chunk in rgb.chunks(3) {
                    bgr.extend_from_slice(&[chunk[2], chunk[1], chunk[0]]);
                }
                Ok((bgr, width, height, PixelFormat::BGR))
            }
            PixelFormat::GRAY8 => {
                let gray = img.into_luma8();
                Ok((gray.into_raw(), width, height, PixelFormat::GRAY8))
            }
            _ => {
                // 不支持的格式，默认转为 RGBA
                let rgba = img.into_rgba8();
                Ok((rgba.into_raw(), width, height, PixelFormat::RGBA))
            }
        }
    }
}

impl Default for MjpegDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MjpegDecoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        // 检查编解码器类型
        if packet.codec_type != CodecType::MJPEG {
            return Err(DecodeError::InvalidParameter(format!(
                "expected MJPEG codec, got {:?}",
                packet.codec_type
            )));
        }

        // 解码 JPEG
        let img = self.decode_jpeg(&packet.data)?;

        // 转换为指定格式
        let (data, width, height, pixel_format) = self.convert_format(img)?;

        // 更新最后一次解码的尺寸
        self.last_width = width;
        self.last_height = height;

        // 创建 MediaFrame
        let frame = MediaFrame::new(
            pixel_format,
            width as i32,
            height as i32,
            bytes::Bytes::from(data),
        )
        .with_timestamps(packet.pts, packet.dts)
        .with_keyframe(true) // MJPEG 每一帧都是关键帧
        .with_duration(packet.dts - packet.pts); // 如果有 duration 信息的话

        Ok(Some(frame))
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        // MJPEG 是无状态解码器，不需要缓存
        Ok(Vec::new())
    }

    fn reset(&mut self) {
        // MJPEG 是无状态解码器，只需要重置尺寸信息
        self.last_width = 0;
        self.last_height = 0;
    }

    fn codec_type(&self) -> CodecType {
        CodecType::MJPEG
    }

    fn name(&self) -> &str {
        "MjpegDecoder"
    }
}

/// 尝试从 JPEG 数据中提取尺寸信息（不解码整个图像）
pub fn get_jpeg_dimensions(data: &[u8]) -> DecodeResult<(u32, u32)> {
    let reader = std::io::Cursor::new(data);
    let format = image::ImageFormat::from_path("dummy.jpg")
        .ok()
        .unwrap_or(image::ImageFormat::Jpeg);
    
    // 使用 image::ImageReader 来读取尺寸
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data));
    reader.set_format(format);
    
    let dims = reader.into_dimensions()
        .map_err(|e| DecodeError::DecodeFailed(format!("failed to get JPEG dimensions: {}", e)))?;
    Ok(dims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_mjpeg_decoder_new() {
        let decoder = MjpegDecoder::new();
        assert_eq!(decoder.codec_type(), CodecType::MJPEG);
        assert_eq!(decoder.name(), "MjpegDecoder");
        assert_eq!(decoder.output_format(), PixelFormat::RGBA);
    }

    #[test]
    fn test_mjpeg_decoder_with_format() {
        let decoder = MjpegDecoder::with_output_format(PixelFormat::RGB);
        assert_eq!(decoder.output_format(), PixelFormat::RGB);
    }

    #[test]
    fn test_mjpeg_decoder_set_format() {
        let mut decoder = MjpegDecoder::new();
        decoder.set_output_format(PixelFormat::BGR);
        assert_eq!(decoder.output_format(), PixelFormat::BGR);
    }

    #[test]
    fn test_decode_invalid_data() {
        let mut decoder = MjpegDecoder::new();
        let packet = MediaPacket::new(CodecType::MJPEG, Bytes::from_static(b"invalid jpeg data"));
        let result = decoder.decode(&packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_wrong_codec() {
        let mut decoder = MjpegDecoder::new();
        let packet = MediaPacket::new(CodecType::H264, Bytes::from_static(b"test"));
        let result = decoder.decode(&packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_flush() {
        let mut decoder = MjpegDecoder::new();
        let result = decoder.flush();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_reset() {
        let mut decoder = MjpegDecoder::new();
        decoder.reset();
        // 重置后应该没有错误
    }

    // 注意：要测试实际解码功能，需要一个有效的 JPEG 文件
    // 这里我们可以使用 image crate 创建一个测试图像
    #[test]
    fn test_decode_valid_jpeg() {
        // 创建一个测试图像并保存为 JPEG
        let img = image::DynamicImage::new_rgb8(100, 100);
        let mut jpeg_data = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut jpeg_data), ImageFormat::Jpeg)
            .unwrap();

        let mut decoder = MjpegDecoder::new();
        let packet = MediaPacket::new(CodecType::MJPEG, Bytes::from(jpeg_data));
        let result = decoder.decode(&packet);

        assert!(result.is_ok());
        let frame = result.unwrap();
        assert!(frame.is_some());
        let frame = frame.unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
        assert_eq!(frame.pixel_format, PixelFormat::RGBA);
    }
}
