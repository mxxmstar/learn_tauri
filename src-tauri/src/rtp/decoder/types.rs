//! 解码器相关枚举定义
//!
//! 对齐 C++ 端的 MediaType、CodecType、PixelFormat 定义

/// 媒体类型（对齐 C++ MediaType）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// 视频流
    Video,
    /// 音频流
    Audio,
    /// 未知类型
    Unknown,
}

impl MediaType {
    /// 从 u32 转换为 MediaType
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => MediaType::Video,
            1 => MediaType::Audio,
            _ => MediaType::Unknown,
        }
    }

    /// 转换为 u32
    pub fn to_u32(&self) -> u32 {
        match self {
            MediaType::Video => 0,
            MediaType::Audio => 1,
            MediaType::Unknown => 0xFF,
        }
    }
}

/// 编码格式（对齐 C++ CodecType）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    /// H.264/AVC
    H264,
    /// H.265/HEVC
    H265,
    /// Motion JPEG
    MJPEG,
    /// MPEG-4 Part 2
    MPEG4,
    /// VP8
    VP8,
    /// VP9
    VP9,
    /// AV1
    AV1,
    /// AAC 音频
    AAC,
    /// MP3 音频
    MP3,
    /// Opus 音频
    OPUS,
    /// G.711 A-law 音频
    G711A,
    /// G.711 μ-law 音频
    G711U,
    /// 未知编码
    Unknown,
}

impl CodecType {
    /// 从 u32 转换为 CodecType
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => CodecType::H264,
            1 => CodecType::H265,
            2 => CodecType::MJPEG,
            3 => CodecType::MPEG4,
            4 => CodecType::VP8,
            5 => CodecType::VP9,
            6 => CodecType::AV1,
            7 => CodecType::AAC,
            8 => CodecType::MP3,
            9 => CodecType::OPUS,
            10 => CodecType::G711A,
            11 => CodecType::G711U,
            _ => CodecType::Unknown,
        }
    }

    /// 转换为 u32
    pub fn to_u32(&self) -> u32 {
        match self {
            CodecType::H264 => 0,
            CodecType::H265 => 1,
            CodecType::MJPEG => 2,
            CodecType::MPEG4 => 3,
            CodecType::VP8 => 4,
            CodecType::VP9 => 5,
            CodecType::AV1 => 6,
            CodecType::AAC => 7,
            CodecType::MP3 => 8,
            CodecType::OPUS => 9,
            CodecType::G711A => 10,
            CodecType::G711U => 11,
            CodecType::Unknown => 0xFF,
        }
    }

    /// 是否为视频编解码器
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            CodecType::H264
                | CodecType::H265
                | CodecType::MJPEG
                | CodecType::MPEG4
                | CodecType::VP8
                | CodecType::VP9
                | CodecType::AV1
        )
    }

    /// 是否为音频编解码器
    pub fn is_audio(&self) -> bool {
        matches!(self, CodecType::AAC | CodecType::MP3 | CodecType::OPUS | CodecType::G711A | CodecType::G711U)
    }
}

/// 像素格式（对齐 C++ PixelFormat）
///
/// 参考：
/// - I420: YUV 4:2:0 planar (Y, U, V 三个平面)
/// - NV12: YUV 4:2:0 semi-planar (Y 平面 + UV 交错平面)
/// - YUY2: YUV 4:2:2 packed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 未知格式
    Unknown,

    /// RGBA 32-bit (R, G, B, A 各 8-bit)
    RGBA,
    /// RGB 24-bit (R, G, B 各 8-bit)
    RGB,
    /// BGRA 32-bit (B, G, R, A 各 8-bit)
    BGRA,
    /// BGR 24-bit (B, G, R 各 8-bit)
    BGR,

    /// YUV 4:2:0 planar (I420/YV12)
    /// 平面 0: Y (width * height)
    /// 平面 1: U (width/2 * height/2)
    /// 平面 2: V (width/2 * height/2)
    YUV420P,
    /// YUV 4:2:0 planar (NV12)
    /// 平面 0: Y (width * height)
    /// 平面 1: UV 交错 (width * height/2)
    NV12,
    /// YUV 4:2:0 planar (NV21)
    /// 平面 0: Y (width * height)
    /// 平面 1: VU 交错 (width * height/2)
    NV21,

    /// YUV 4:2:2 planar
    YUV422P,
    /// YUV 4:2:2 packed (YUY2/YUYV)
    YUY2,
    /// YUV 4:2:2 packed (UYVY)
    UYVY,

    /// YUV 4:4:4 planar
    YUV444P,

    /// Grayscale 8-bit
    GRAY8,
    /// Grayscale 16-bit
    GRAY16,

    /// Monochrome (1-bit)
    MONO,
}

impl PixelFormat {
    /// 从 u32 转换为 PixelFormat
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => PixelFormat::Unknown,
            1 => PixelFormat::RGBA,
            2 => PixelFormat::RGB,
            3 => PixelFormat::BGRA,
            4 => PixelFormat::BGR,
            5 => PixelFormat::YUV420P,
            6 => PixelFormat::NV12,
            7 => PixelFormat::NV21,
            8 => PixelFormat::YUV422P,
            9 => PixelFormat::YUY2,
            10 => PixelFormat::UYVY,
            11 => PixelFormat::YUV444P,
            12 => PixelFormat::GRAY8,
            13 => PixelFormat::GRAY16,
            14 => PixelFormat::MONO,
            _ => PixelFormat::Unknown,
        }
    }

    /// 转换为 u32
    pub fn to_u32(&self) -> u32 {
        match self {
            PixelFormat::Unknown => 0,
            PixelFormat::RGBA => 1,
            PixelFormat::RGB => 2,
            PixelFormat::BGRA => 3,
            PixelFormat::BGR => 4,
            PixelFormat::YUV420P => 5,
            PixelFormat::NV12 => 6,
            PixelFormat::NV21 => 7,
            PixelFormat::YUV422P => 8,
            PixelFormat::YUY2 => 9,
            PixelFormat::UYVY => 10,
            PixelFormat::YUV444P => 11,
            PixelFormat::GRAY8 => 12,
            PixelFormat::GRAY16 => 13,
            PixelFormat::MONO => 14,
        }
    }

    /// 是否为 RGB 格式
    pub fn is_rgb(&self) -> bool {
        matches!(self, PixelFormat::RGBA | PixelFormat::RGB | PixelFormat::BGRA | PixelFormat::BGR)
    }

    /// 是否为 YUV 格式
    pub fn is_yuv(&self) -> bool {
        matches!(
            self,
            PixelFormat::YUV420P
                | PixelFormat::NV12
                | PixelFormat::NV21
                | PixelFormat::YUV422P
                | PixelFormat::YUY2
                | PixelFormat::UYVY
                | PixelFormat::YUV444P
        )
    }

    /// 是否为灰度格式
    pub fn is_gray(&self) -> bool {
        matches!(self, PixelFormat::GRAY8 | PixelFormat::GRAY16 | PixelFormat::MONO)
    }

    /// 获取每个像素的字节数（对于 packed 格式）
    /// 对于 planar 格式，返回 0（需要分别计算每个平面）
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            PixelFormat::RGBA | PixelFormat::BGRA => 4,
            PixelFormat::RGB | PixelFormat::BGR => 3,
            PixelFormat::YUY2 | PixelFormat::UYVY => 2,
            _ => 0,
        }
    }

    /// 计算帧大小（字节）
    /// 对于 planar 格式，需要分别计算每个平面
    pub fn frame_size(&self, width: u32, height: u32) -> u32 {
        match self {
            PixelFormat::RGBA | PixelFormat::BGRA => width * height * 4,
            PixelFormat::RGB | PixelFormat::BGR => width * height * 3,
            PixelFormat::YUV420P | PixelFormat::NV12 | PixelFormat::NV21 => {
                // Y: width * height, UV: width * height / 2
                width * height * 3 / 2
            }
            PixelFormat::YUV422P => {
                // Y: width * height, U: width * height / 2, V: width * height / 2
                width * height * 2
            }
            PixelFormat::YUY2 | PixelFormat::UYVY => width * height * 2,
            PixelFormat::YUV444P => width * height * 3,
            PixelFormat::GRAY8 => width * height,
            PixelFormat::GRAY16 => width * height * 2,
            _ => 0,
        }
    }
}

/// 后端引擎句柄（对齐 C++ BackendHandle）
///
/// 用于传递后端引擎的指针或标识符
pub type BackendHandle = usize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_conversion() {
        assert_eq!(MediaType::from_u32(0), MediaType::Video);
        assert_eq!(MediaType::from_u32(1), MediaType::Audio);
        assert_eq!(MediaType::from_u32(99), MediaType::Unknown);

        assert_eq!(MediaType::Video.to_u32(), 0);
        assert_eq!(MediaType::Audio.to_u32(), 1);
    }

    #[test]
    fn test_codec_type() {
        assert_eq!(CodecType::from_u32(0), CodecType::H264);
        assert_eq!(CodecType::from_u32(2), CodecType::MJPEG);
        assert_eq!(CodecType::H264.to_u32(), 0);

        assert!(CodecType::H264.is_video());
        assert!(CodecType::AAC.is_audio());
    }

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::from_u32(1), PixelFormat::RGBA);
        assert_eq!(PixelFormat::RGBA.to_u32(), 1);

        assert!(PixelFormat::RGBA.is_rgb());
        assert!(PixelFormat::YUV420P.is_yuv());
        assert!(PixelFormat::GRAY8.is_gray());

        assert_eq!(PixelFormat::RGBA.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::RGB.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::YUV420P.bytes_per_pixel(), 0);

        assert_eq!(PixelFormat::RGBA.frame_size(1920, 1080), 1920 * 1080 * 4);
        assert_eq!(
            PixelFormat::YUV420P.frame_size(1920, 1080),
            1920 * 1080 * 3 / 2
        );
    }
}

/// 音频采样格式（对齐 C++ SampleFormat）
///
/// 参考 FFmpeg 的 AVSampleFormat：
/// - 非 planar 格式：交错存储（interleaved）
/// - planar 格式：平面存储（每个声道独立存储）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 未知格式
    Unknown,

    /// 8-bit unsigned integer (AV_SAMPLE_FMT_U8)
    U8,
    /// 16-bit signed integer (AV_SAMPLE_FMT_S16)
    S16,
    /// 32-bit signed integer (AV_SAMPLE_FMT_S32)
    S32,
    /// 32-bit floating point (AV_SAMPLE_FMT_FLT)
    F32,
    /// 64-bit floating point (AV_SAMPLE_FMT_DBL)
    F64,

    /// 8-bit unsigned integer planar (AV_SAMPLE_FMT_U8P)
    U8P,
    /// 16-bit signed integer planar (AV_SAMPLE_FMT_S16P)
    S16P,
    /// 32-bit signed integer planar (AV_SAMPLE_FMT_S32P)
    S32P,
    /// 32-bit floating point planar (AV_SAMPLE_FMT_FLTP)
    F32P,
    /// 64-bit floating point planar (AV_SAMPLE_FMT_DBLP)
    F64P,
}

impl SampleFormat {
    /// 从 u32 转换为 SampleFormat
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => SampleFormat::Unknown,
            1 => SampleFormat::U8,
            2 => SampleFormat::S16,
            3 => SampleFormat::S32,
            4 => SampleFormat::F32,
            5 => SampleFormat::F64,
            6 => SampleFormat::U8P,
            7 => SampleFormat::S16P,
            8 => SampleFormat::S32P,
            9 => SampleFormat::F32P,
            10 => SampleFormat::F64P,
            _ => SampleFormat::Unknown,
        }
    }

    /// 转换为 u32
    pub fn to_u32(&self) -> u32 {
        match self {
            SampleFormat::Unknown => 0,
            SampleFormat::U8 => 1,
            SampleFormat::S16 => 2,
            SampleFormat::S32 => 3,
            SampleFormat::F32 => 4,
            SampleFormat::F64 => 5,
            SampleFormat::U8P => 6,
            SampleFormat::S16P => 7,
            SampleFormat::S32P => 8,
            SampleFormat::F32P => 9,
            SampleFormat::F64P => 10,
        }
    }

    /// 是否为 planar 格式（平面存储）
    pub fn is_planar(&self) -> bool {
        matches!(
            self,
            SampleFormat::U8P
                | SampleFormat::S16P
                | SampleFormat::S32P
                | SampleFormat::F32P
                | SampleFormat::F64P
        )
    }

    /// 是否为 packed 格式（交错存储）
    pub fn is_packed(&self) -> bool {
        !self.is_planar()
    }

    /// 获取每个采样点的字节数
    pub fn bytes_per_sample(&self) -> u32 {
        match self {
            SampleFormat::U8 | SampleFormat::U8P => 1,
            SampleFormat::S16 | SampleFormat::S16P => 2,
            SampleFormat::S32 | SampleFormat::S32P | SampleFormat::F32 | SampleFormat::F32P => 4,
            SampleFormat::F64 | SampleFormat::F64P => 8,
            SampleFormat::Unknown => 0,
        }
    }

    /// 获取对应的 planar 格式
    pub fn to_planar(&self) -> Option<SampleFormat> {
        match self {
            SampleFormat::U8 => Some(SampleFormat::U8P),
            SampleFormat::S16 => Some(SampleFormat::S16P),
            SampleFormat::S32 => Some(SampleFormat::S32P),
            SampleFormat::F32 => Some(SampleFormat::F32P),
            SampleFormat::F64 => Some(SampleFormat::F64P),
            _ => None,
        }
    }

    /// 获取对应的 packed 格式
    pub fn to_packed(&self) -> Option<SampleFormat> {
        match self {
            SampleFormat::U8P => Some(SampleFormat::U8),
            SampleFormat::S16P => Some(SampleFormat::S16),
            SampleFormat::S32P => Some(SampleFormat::S32),
            SampleFormat::F32P => Some(SampleFormat::F32),
            SampleFormat::F64P => Some(SampleFormat::F64),
            _ => None,
        }
    }

    /// 计算一帧音频数据的字节大小
    ///
    /// # Arguments
    /// * `nb_samples` - 每声道的采样点数
    /// * `channels` - 声道数
    pub fn frame_size(&self, nb_samples: u32, channels: u32) -> u32 {
        if self.is_planar() {
            // planar 格式：每个声道独立存储
            self.bytes_per_sample() * nb_samples * channels
        } else {
            // packed 格式：所有声道交错存储
            self.bytes_per_sample() * nb_samples * channels
        }
    }
}

#[cfg(test)]
mod sample_format_tests {
    use super::*;

    #[test]
    fn test_sample_format_conversion() {
        assert_eq!(SampleFormat::from_u32(0), SampleFormat::Unknown);
        assert_eq!(SampleFormat::from_u32(1), SampleFormat::U8);
        assert_eq!(SampleFormat::from_u32(2), SampleFormat::S16);
        assert_eq!(SampleFormat::from_u32(6), SampleFormat::U8P);
        assert_eq!(SampleFormat::from_u32(99), SampleFormat::Unknown);

        assert_eq!(SampleFormat::Unknown.to_u32(), 0);
        assert_eq!(SampleFormat::U8.to_u32(), 1);
        assert_eq!(SampleFormat::S16.to_u32(), 2);
        assert_eq!(SampleFormat::F32P.to_u32(), 9);
    }

    #[test]
    fn test_sample_format_planar() {
        assert!(!SampleFormat::U8.is_planar());
        assert!(!SampleFormat::S16.is_planar());
        assert!(!SampleFormat::F32.is_planar());

        assert!(SampleFormat::U8P.is_planar());
        assert!(SampleFormat::S16P.is_planar());
        assert!(SampleFormat::F32P.is_planar());
        assert!(SampleFormat::F64P.is_planar());

        assert!(SampleFormat::U8.is_packed());
        assert!(!SampleFormat::U8P.is_packed());
    }

    #[test]
    fn test_sample_format_bytes_per_sample() {
        assert_eq!(SampleFormat::U8.bytes_per_sample(), 1);
        assert_eq!(SampleFormat::S16.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::S32.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::F32.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::F64.bytes_per_sample(), 8);

        assert_eq!(SampleFormat::U8P.bytes_per_sample(), 1);
        assert_eq!(SampleFormat::S16P.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::F32P.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::F64P.bytes_per_sample(), 8);
    }

    #[test]
    fn test_sample_format_conversion_planar_packed() {
        assert_eq!(SampleFormat::U8.to_planar(), Some(SampleFormat::U8P));
        assert_eq!(SampleFormat::S16.to_planar(), Some(SampleFormat::S16P));
        assert_eq!(SampleFormat::F32.to_planar(), Some(SampleFormat::F32P));

        assert_eq!(SampleFormat::U8P.to_packed(), Some(SampleFormat::U8));
        assert_eq!(SampleFormat::S16P.to_packed(), Some(SampleFormat::S16));
        assert_eq!(SampleFormat::F32P.to_packed(), Some(SampleFormat::F32));

        assert_eq!(SampleFormat::Unknown.to_planar(), None);
        assert_eq!(SampleFormat::Unknown.to_packed(), None);
    }

    #[test]
    fn test_sample_format_frame_size() {
        // S16 格式，1024 采样点，2 声道
        // packed: 2 bytes * 1024 * 2 = 4096
        assert_eq!(SampleFormat::S16.frame_size(1024, 2), 4096);

        // S16P 格式，1024 采样点，2 声道
        // planar: 2 bytes * 1024 * 2 = 4096 (每个声道 2048 字节)
        assert_eq!(SampleFormat::S16P.frame_size(1024, 2), 4096);

        // F32 格式，512 采样点，1 声道
        assert_eq!(SampleFormat::F32.frame_size(512, 1), 2048);
    }
}
