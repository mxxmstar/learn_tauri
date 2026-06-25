# RTP Decoder 模块设计指南

## 目录

- [模块概述](#模块概述)
- [设计目标](#设计目标)
- [文件结构](#文件结构)
- [核心概念](#核心概念)
- [枚举定义](#枚举定义)
- [数据结构](#数据结构)
- [Decoder Trait](#decoder-trait)
- [解码器实现](#解码器实现)
- [使用示例](#使用示例)
- [Feature Flags](#feature-flags)
- [与 C++ 端对齐](#与-c-端对齐)
- [后续扩展](#后续扩展)

---

## 模块概述

`rtp::decoder` 模块提供了统一的多媒体解码器接口，支持多种编解码器（H264、H265、MJPEG 等），对齐 C++ 端的 `MediaPacket` 和 `MediaFrame` 设计。

**位置**：`src-tauri/src/rtp/decoder/`

**功能**：
- 统一的 `Decoder` trait 定义
- 多种像素格式支持（RGBA、RGB、YUV420P、NV12 等）
- MJPEG 纯 Rust 解码实现（基于 `image` crate）
- H264/H265 预留 FFI 接口（对接 C++/FFmpeg）
- 解码器工厂和配置系统

---

## 设计目标

1. **对齐 C++ 端定义**：Rust 端的 `MediaPacket` 和 `MediaFrame` 完全对齐 C++ 端的类定义
2. **统一接口**：所有解码器实现统一的 `Decoder` trait
3. **多后端支持**：通过 feature flag 切换纯 Rust 实现和 FFI 实现
4. **易于扩展**：新增编解码器只需实现 `Decoder` trait

---

## 文件结构

```
src-tauri/src/rtp/decoder/
├── mod.rs          # 模块导出、解码器工厂、DecoderConfig
├── types.rs        # 枚举定义：MediaType、CodecType、PixelFormat
├── frame.rs        # 数据结构：MediaPacket、MediaFrame
├── trait_.rs       # Decoder trait、DecodeError、StatsDecoder
├── mjpeg.rs        # MJPEG 解码器（纯 Rust，使用 image crate）
├── h264.rs         # H264 解码器接口（预留，FFI 实现）
├── h265.rs         # H265 解码器接口（预留，FFI 实现）
└── ffi.rs          # FFI 接口定义（预留，对接 C++/FFmpeg）
```

---

## 核心概念

### 数据流

```
RTP 包 → RTP 重组器 → MediaPacket → Decoder::decode() → MediaFrame → 渲染/处理
```

### 关键类型

- **MediaPacket**：编码数据包（输入到解码器）
- **MediaFrame**：解码后帧（解码器输出）
- **Decoder trait**：统一的解码器接口
- **CodecType**：编解码器类型枚举
- **PixelFormat**：像素格式枚举

---

## 枚举定义

### MediaType（媒体类型）

```rust
pub enum MediaType {
    Video,      // 视频流
    Audio,      // 音频流
    Unknown,    // 未知类型
}
```

**对齐 C++**：
```cpp
enum class MediaType {
    VIDEO = 0,
    AUDIO = 1,
    UNKNOWN = 0xFF
};
```

### CodecType（编码格式）

```rust
pub enum CodecType {
    H264,       // H.264/AVC
    H265,       // H.265/HEVC
    MJPEG,      // Motion JPEG
    MPEG4,      // MPEG-4 Part 2
    VP8,        // VP8
    VP9,        // VP9
    AV1,        // AV1
    AAC,        // AAC 音频
    G711A,      // G.711 A-law 音频
    G711U,      // G.711 μ-law 音频
    Unknown,    // 未知编码
}
```

**方法**：
- `is_video()`：是否为视频编解码器
- `is_audio()`：是否为音频编解码器
- `from_u32(value)`：从 u32 转换
- `to_u32()`：转换为 u32

### PixelFormat（像素格式）

```rust
pub enum PixelFormat {
    Unknown,    // 未知格式

    // RGB 格式
    RGBA,       // RGBA 32-bit
    RGB,        // RGB 24-bit
    BGRA,       // BGRA 32-bit
    BGR,        // BGR 24-bit

    // YUV 格式
    YUV420P,    // YUV 4:2:0 planar (I420/YV12)
    NV12,       // YUV 4:2:0 semi-planar
    NV21,       // YUV 4:2:0 semi-planar (VU 交错)
    YUV422P,    // YUV 4:2:2 planar
    YUY2,       // YUV 4:2:2 packed (YUYV)
    UYVY,       // YUV 4:2:2 packed
    YUV444P,    // YUV 4:4:4 planar

    // 灰度格式
    GRAY8,      // Grayscale 8-bit
    GRAY16,     // Grayscale 16-bit
    MONO,       // Monochrome 1-bit
}
```

**方法**：
- `is_rgb()`：是否为 RGB 格式
- `is_yuv()`：是否为 YUV 格式
- `is_gray()`：是否为灰度格式
- `bytes_per_pixel()`：每个像素的字节数（packed 格式）
- `frame_size(width, height)`：计算帧大小（字节）

**对齐 C++**：
```cpp
enum class PixelFormat {
    kUnknown = 0,
    kRGBA = 1,
    kRGB = 2,
    kBGRA = 3,
    kBGR = 4,
    kYUV420P = 5,
    kNV12 = 6,
    kNV21 = 7,
    kYUV422P = 8,
    kYUY2 = 9,
    kUYVY = 10,
    kYUV444P = 11,
    kGRAY8 = 12,
    kGRAY16 = 13,
    kMONO = 14
};
```

---

## 数据结构

### MediaPacket（编码数据包）

**对齐 C++**：
```cpp
class MediaPacket {
public:
    MediaType  type{MediaType::UNKNOWN};
    CodecType  codec{CodecType::UNKNOWN};
    int64_t    pts{0};
    int64_t    dts{0};
    bool       keyframe{false};
    std::shared_ptr<IMediaBuffer> buffer;
    BackendHandle backend;
};
```

**Rust 实现**：
```rust
pub struct MediaPacket {
    pub media_type: MediaType,      // 媒体流类型
    pub codec_type: CodecType,      // 编码格式
    pub pts: i64,                   // 显示时间戳（微秒）
    pub dts: i64,                   // 解码时间戳（微秒）
    pub keyframe: bool,             // 是否为关键帧
    pub data: Bytes,                // 编码数据载荷
    pub backend: Option<BackendHandle>, // 后端引擎句柄
}
```

**构造方法**：
```rust
// 创建新的 MediaPacket
let packet = MediaPacket::new(CodecType::H264, data)
    .with_timestamps(1000, 1000)  // 设置 pts 和 dts
    .with_keyframe(true)          // 标记为关键帧
    .with_backend(0);            // 设置后端句柄
```

**从 RTP 重组器转换**：
```rust
// JpegFrame → MediaPacket
let packet: MediaPacket = jpeg_frame.into();

// H264AccessUnit → MediaPacket
let packet: MediaPacket = h264_access_unit.into();

// H265AccessUnit → MediaPacket
let packet: MediaPacket = h265_access_unit.into();
```

### MediaFrame（解码后帧）

**对齐 C++**：
```cpp
class MediaFrame {
public:
    MediaType    type{MediaType::VIDEO};
    PixelFormat pixel_format{PixelFormat::kUnknown};
    int32_t     width{0};
    int32_t     height{0};
    int32_t     stride[4] {0};
    int32_t     plane_offset[4] {0};
    int32_t     plane_count{0};
    int64_t     pts{0};
    int64_t     dts{0};
    int64_t     duration{0};
    bool        keyframe{false};
    std::shared_ptr<IMediaBuffer> buffer;
    BackendHandle backend;
};
```

**Rust 实现**：
```rust
pub struct MediaFrame {
    pub media_type: MediaType,
    pub pixel_format: PixelFormat,
    pub width: i32,
    pub height: i32,
    pub stride: [i32; 4],           // 平面行跨度（字节）
    pub plane_offset: [i32; 4],     // 平面数据偏移（字节）
    pub plane_count: i32,           // 平面数量
    pub pts: i64,
    pub dts: i64,
    pub duration: i64,
    pub keyframe: bool,
    pub data: Bytes,
    pub backend: Option<BackendHandle>,
}
```

**平面数据布局**：

| 像素格式 | plane_count | stride | plane_offset |
|----------|-------------|--------|--------------|
| RGBA | 1 | [width*4, 0, 0, 0] | [0, 0, 0, 0] |
| RGB | 1 | [width*3, 0, 0, 0] | [0, 0, 0, 0] |
| YUV420P | 3 | [width, width/2, width/2, 0] | [0, Y, Y+U, 0] |
| NV12 | 2 | [width, width, 0, 0] | [0, Y, 0, 0] |
| GRAY8 | 1 | [width, 0, 0, 0] | [0, 0, 0, 0] |

**方法**：
```rust
// 创建新的 MediaFrame
let frame = MediaFrame::new(PixelFormat::RGBA, 1920, 1080, data)
    .with_timestamps(1000, 1000)
    .with_duration(33333)  // 30 FPS
    .with_keyframe(true);

// 获取指定平面的数据
if let Some(y_plane) = frame.plane_data(0) {
    // 处理 Y 平面
}

// 获取帧大小
let size = frame.frame_size();
```

---

## Decoder Trait

### 定义

```rust
pub trait Decoder: Send {
    /// 解码一个编码包
    ///
    /// 对于需要多个包才能解码出一帧的编解码器（如 H264/H265），
    /// 此函数可能返回 None，直到有足够的数据
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>;

    /// 刷新解码器，返回所有缓存的帧
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>>;

    /// 重置解码器状态
    fn reset(&mut self);

    /// 获取此解码器支持的编解码器类型
    fn codec_type(&self) -> CodecType;

    /// 获取解码器名称
    fn name(&self) -> &str;

    /// 获取解码器信息（默认实现）
    fn info(&self) -> DecoderInfo {
        DecoderInfo {
            name: self.name().to_string(),
            codec_type: self.codec_type(),
        }
    }
}
```

### 方法说明

#### decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>

**功能**：解码一个编码包

**返回值**：
- `Ok(Some(frame))`：成功解码出一帧
- `Ok(None)`：数据已缓存，尚未有足够数据解码出完整帧（如 H264 需要多个 NAL 单元）
- `Err(e)`：解码失败

**注意**：
- 对于 MJPEG（每帧独立），每次调用都返回 `Some(frame)`
- 对于 H264/H265（需要多个 NAL 单元），可能返回 `None`

#### flush(&mut self) -> DecodeResult<Vec<MediaFrame>>

**功能**：刷新解码器，返回所有缓存的帧

**使用场景**：
- 流结束时调用
- 切换码流时调用

#### reset(&mut self)

**功能**：重置解码器状态

**使用场景**：
- 解码新流时调用
- 发生错误需要恢复时调用

### 错误类型

```rust
pub enum DecodeError {
    UnsupportedCodec(CodecType),    // 不支持的编解码器
    DecodeFailed(String),           // 解码失败
    InvalidData(String),            // 无效的数据
    InternalError(String),          // 内部错误
    InvalidParameter(String),       // 参数错误
    BufferOverflow,                 // 缓冲区溢出
    Timeout,                        // 超时
    Underlying(Box<dyn Error>),    // 底层错误
}
```

---

## 解码器实现

### MjpegDecoder（MJPEG 解码器）

**文件**：`mjpeg.rs`

**特性**：
- 纯 Rust 实现（使用 `image` crate）
- 支持多种输出格式：RGBA、RGB、BGRA、BGR、GRAY8
- 自动从 JPEG 数据提取尺寸信息

**实现**：
```rust
pub struct MjpegDecoder {
    output_format: PixelFormat,
}

impl MjpegDecoder {
    /// 创建新的 MJPEG 解码器（默认输出 RGBA）
    pub fn new() -> Self {
        Self {
            output_format: PixelFormat::RGBA,
        }
    }

    /// 创建指定输出格式的 MJPEG 解码器
    pub fn with_output_format(format: PixelFormat) -> Self {
        Self {
            output_format: format,
        }
    }
}

impl Decoder for MjpegDecoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        // 1. 使用 image crate 解码 JPEG
        let img = image::load_from_memory(&packet.data)
            .map_err(|e| DecodeError::DecodeFailed(e.to_string()))?;

        // 2. 转换为目标像素格式
        let (width, height, data) = match self.output_format {
            PixelFormat::RGBA => {
                let rgba = img.to_rgba8();
                (rgba.width(), rgba.height(), rgba.into_raw())
            }
            PixelFormat::RGB => {
                let rgb = img.to_rgb8();
                (rgb.width(), rgb.height(), rgb.into_raw())
            }
            // ... 其他格式
        };

        // 3. 创建 MediaFrame
        let frame = MediaFrame::new(
            self.output_format,
            width as i32,
            height as i32,
            Bytes::from(data),
        )
        .with_timestamps(packet.pts, packet.dts)
        .with_keyframe(packet.keyframe);

        Ok(Some(frame))
    }

    // ... 其他方法
}
```

**辅助函数**：
```rust
/// 尝试从 JPEG 数据中提取尺寸信息（不解码整个图像）
pub fn get_jpeg_dimensions(data: &[u8]) -> DecodeResult<(u32, u32)> {
    let reader = image::ImageReader::new(std::io::Cursor::new(data));
    // ...
}
```

### H264Decoder（H264 解码器，预留）

**文件**：`h264.rs`

**状态**：接口已定义，等待 FFI 实现

**接口**：
```rust
pub struct H264Decoder {
    output_format: PixelFormat,
    // FFI 相关字段
}

impl H264Decoder {
    pub fn new() -> DecodeResult<Self> { /* FFI 初始化 */ }
    pub fn with_output_format(format: PixelFormat) -> DecodeResult<Self> { /* ... */ }
}

impl Decoder for H264Decoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        // FFI 调用 C++ 解码器
    }
}
```

### H265Decoder（H265 解码器，预留）

**文件**：`h265.rs`

**状态**：接口已定义，等待 FFI 实现

### FfiDecoder（FFI 解码器，预留）

**文件**：`ffi.rs`

**功能**：对接 C++/FFmpeg 的通用 FFI 解码器

**C 风格接口定义**：
```rust
#[repr(C)]
pub struct DecoderHandle {
    pub ptr: *mut c_void,
}

#[repr(C)]
pub struct DecodedFrame {
    pub pixel_format: u32,
    pub width: i32,
    pub height: i32,
    pub stride: [i32; 4],
    pub plane_offset: [i32; 4],
    pub plane_count: i32,
    pub pts: i64,
    pub dts: i64,
    pub data_ptr: *const u8,
    pub data_size: usize,
}

extern "C" {
    pub fn decoder_create(codec: u32, output_format: u32) -> DecoderHandle;
    pub fn decoder_decode(handle: DecoderHandle, data: *const u8, size: usize, pts: i64, dts: i64) -> FfiErrorCode;
    pub fn decoder_get_frame(handle: DecoderHandle, frame: *mut DecodedFrame) -> bool;
    pub fn decoder_flush(handle: DecoderHandle);
    pub fn decoder_destroy(handle: DecoderHandle);
}
```

---

## 使用示例

### 示例 1：创建解码器并解码 MJPEG

```rust
use rtp::decoder::{
    create_decoder, create_decoder_with_format,
    CodecType, PixelFormat, MediaPacket
};
use bytes::Bytes;

// 方法 1：使用工厂函数创建（默认输出 RGBA）
let mut decoder = create_decoder(CodecType::MJPEG).unwrap();

// 方法 2：指定输出格式
let mut decoder = create_decoder_with_format(
    CodecType::MJPEG,
    PixelFormat::RGB,  // 输出 RGB 24-bit
).unwrap();

// 解码
let jpeg_data = Bytes::from(/* JPEG 二进制数据 */);
let packet = MediaPacket::new(CodecType::MJPEG, jpeg_data)
    .with_timestamps(1000, 1000)
    .with_keyframe(true);

match decoder.decode(&packet) {
    Ok(Some(frame)) => {
        println!("Decoded frame: {}x{}", frame.width, frame.height);
        println!("Pixel format: {:?}", frame.pixel_format);
        println!("Data size: {} bytes", frame.data.len());
    }
    Ok(None) => {
        // MJPEG 不会返回 None
    }
    Err(e) => {
        eprintln!("Decode error: {}", e);
    }
}
```

### 示例 2：使用 DecoderConfig

```rust
use rtp::decoder::{DecoderConfig, CodecType, PixelFormat};

let config = DecoderConfig::new(CodecType::MJPEG)
    .with_output_format(PixelFormat::RGBA)
    .with_stats(true);  // 启用统计信息

let mut decoder = config.create_decoder().unwrap();

// 解码...
```

### 示例 3：从 RTP 重组器到解码器

```rust
use rtp::{mjpeg::MjpegReassembler, decoder::{create_decoder, CodecType}};

// 1. 创建 RTP 重组器
let mut reassembler = MjpegReassembler::new();

// 2. 创建解码器
let mut decoder = create_decoder(CodecType::MJPEG).unwrap();

// 3. 接收 RTP 包并重组
for rtp_packet in received_rtp_packets {
    if let Some(jpeg_frame) = reassembler.push(rtp_packet)? {
        // 4. 转换为 MediaPacket
        let packet: MediaPacket = jpeg_frame.into();

        // 5. 解码
        if let Some(frame) = decoder.decode(&packet)? {
            // 6. 使用解码后的帧
            render_frame(&frame);
        }
    }
}

// 7. 刷新解码器
let remaining_frames = decoder.flush()?;
```

### 示例 4：使用 StatsDecoder 统计解码性能

```rust
use rtp::decoder::{StatsDecoder, create_decoder, CodecType};

let decoder = create_decoder(CodecType::MJPEG).unwrap();
let mut stats_decoder = StatsDecoder::new(decoder);

// 解码 100 个包
for i in 0..100 {
    let packet = /* ... */;
    let _ = stats_decoder.decode(&packet);
}

// 查看统计信息
let stats = stats_decoder.stats();
println!("Packets in: {}", stats.packets_in);
println!("Frames out: {}", stats.frames_out);
println!("Decode errors: {}", stats.decode_errors);
println!("Success rate: {:.2}%",
    stats.frames_out as f32 / stats.packets_in as f32 * 100.0);
```

### 示例 5：检查编解码器支持

```rust
use rtp::decoder::{is_codec_supported, supported_codecs, CodecType};

// 检查单个编解码器
if is_codec_supported(CodecType::MJPEG) {
    println!("MJPEG is supported");
}

// 获取所有支持的编解码器
let codecs = supported_codecs();
println!("Supported codecs: {:?}", codecs);
```

---

## Feature Flags

### 定义

```toml
[features]
default = ["decoder-rust"]
decoder-rust = []  # 纯 Rust 解码器实现（MJPEG）
decoder-ffi = []   # FFI 解码器实现（对接 C++/FFmpeg，支持 H264/H265）
```

### 使用

**默认（纯 Rust）**：
```toml
# Cargo.toml
[dependencies]
your-crate = { path = "../path" }  # 默认启用 decoder-rust
```

**启用 FFI**：
```toml
[dependencies]
your-crate = { path = "../path", features = ["decoder-ffi"] }
```

**同时启用**：
```toml
[dependencies]
your-crate = { path = "../path", features = ["decoder-rust", "decoder-ffi"] }
```

### 条件编译

```rust
// 只在 decoder-rust feature 启用时编译
#[cfg(feature = "decoder-rust")]
mod mjpeg;

// 只在 decoder-ffi feature 启用时编译
#[cfg(feature = "decoder-ffi")]
mod h264;
```

---

## 与 C++ 端对齐

### 数据结构对齐

| C++ 端 | Rust 端 | 说明 |
|--------|---------|------|
| `MediaType` | `MediaType` | 完全一致 |
| `CodecType` | `CodecType` | 完全一致 |
| `PixelFormat` | `PixelFormat` | 完全一致 |
| `MediaPacket` | `MediaPacket` | 对齐，`shared_ptr<IMediaBuffer>` → `Bytes` |
| `MediaFrame` | `MediaFrame` | 对齐，`shared_ptr<IMediaBuffer>` → `Bytes` |
| `BackendHandle` | `BackendHandle` (`usize`) | 完全一致 |

### 接口对齐

**C++ 端**：
```cpp
class IDecoder {
public:
    virtual MediaFrame* decode(const MediaPacket* packet) = 0;
    virtual void flush() = 0;
    virtual void reset() = 0;
    virtual CodecType codecType() const = 0;
    virtual const char* name() const = 0;
};
```

**Rust 端**：
```rust
pub trait Decoder: Send {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>;
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>>;
    fn reset(&mut self);
    fn codec_type(&self) -> CodecType;
    fn name(&self) -> &str;
}
```

**差异**：
1. Rust 返回 `Result`，C++ 使用异常或错误码
2. Rust 的 `decode` 返回 `Option<MediaFrame>`，支持多次调用才输出一帧（如 H264）
3. Rust 的 `flush` 返回所有缓存帧，C++ 可能需要多次调用

---

## 后续扩展

### 1. 实现 H264/H265 解码器

**方案 A：FFI 对接 C++/FFmpeg**
- 在 `h264.rs` 和 `h265.rs` 中实现 FFI 调用
- 使用 `ffi.rs` 中定义的 C 风格接口
- 需要编译 C++ 解码库并链接

**方案 B：纯 Rust 实现**
- 使用 `rav1e`（AV1）、`vpx`（VP8/VP9）等纯 Rust 编解码库
- 或使用 `ffmpeg-next`（FFmpeg Rust 绑定）

### 2. 添加音频解码支持

**新增文件**：
- `decoder/audio.rs`：音频解码器 trait
- `decoder/aac.rs`：AAC 解码器
- `decoder/g711.rs`：G.711 解码器

**音频帧结构**：
```rust
pub struct AudioFrame {
    pub sample_format: SampleFormat,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,  // 或 f32
    pub pts: i64,
}
```

### 3. 硬件加速

**支持硬件加速 API**：
- Windows：DXVA2、D3D11VA
- Linux：VAAPI、VDPAU
- macOS：VideoToolbox

**实现**：
```rust
pub trait HardwareAccelerator {
    fn create_decoder(&self, codec: CodecType) -> DecodeResult<Box<dyn Decoder>>;
}
```

### 4. 性能优化

- 使用 GPU 解码（CUDA、OpenCL）
- 零拷贝：避免 `Bytes` 拷贝，直接使用 GPU 内存
- 并行解码：多个解码器实例并行解码

---

## 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行 decoder 模块测试
cargo test rtp::decoder

# 运行特定测试
cargo test test_create_mjpeg_decoder
```

### 测试覆盖

- `types.rs`：枚举转换测试
- `frame.rs`：MediaPacket/MediaFrame 创建和平面数据测试
- `trait_.rs`：Decoder trait、StatsDecoder 测试
- `mjpeg.rs`：MJPEG 解码器测试
- `mod.rs`：工厂函数、配置测试

---

## 常见问题

### Q1：为什么 `trait.rs` 命名为 `trait_.rs`？

**A**：`trait` 是 Rust 关键字，不能用作文件名。使用 `trait_.rs` 避免冲突。

### Q2：如何添加新的编解码器？

**A**：
1. 在 `types.rs` 中添加 `CodecType` 枚举值
2. 创建新的解码器文件（如 `decoder/my_codec.rs`）
3. 实现 `Decoder` trait
4. 在 `mod.rs` 中注册解码器工厂

### Q3：为什么 `MediaPacket` 和 `MediaFrame` 使用 `Bytes`？

**A**：`Bytes` 是零拷贝的引用计数缓冲区，适合在网络编程中使用，避免不必要的数据拷贝。

### Q4：如何对接 C++ 解码器？

**A**：参考 `ffi.rs` 中的接口定义，实现 FFI 调用。步骤如下：
1. 编写 C++ 解码器包装器（导出 C 风格函数）
2. 编译为静态库或动态库
3. 在 Rust 中使用 `extern "C"` 声明函数
4. 实现 `Decoder` trait，内部调用 FFI 函数

---

## 总结

`rtp::decoder` 模块提供了统一、可扩展的多媒体解码器接口，完全对齐 C++ 端设计。当前已实现：

- ✅ 完整的 `Decoder` trait 定义
- ✅ `MediaPacket` 和 `MediaFrame` 数据结构
- ✅ MJPEG 纯 Rust 解码器（可用）
- ✅ H264/H265 接口预留（等待 FFI 实现）
- ✅ FFI 接口定义
- ✅ 解码器工厂和配置系统

**下一步**：
1. 实现 H264/H265 FFI 解码器
2. 添加音频解码支持
3. 优化性能

---

**文档版本**：1.0  
**更新日期**：2026-06-25  
**作者**：CodeBuddy AI
