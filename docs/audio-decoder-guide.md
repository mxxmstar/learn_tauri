# 音频解码器使用指南

## 概述

本文档介绍如何在 RTP 解码器模块中使用音频解码功能。目前支持以下音频编解码器：

- **G.711 A-law** (纯 Rust 实现，无需额外依赖)
- **G.711 μ-law** (纯 Rust 实现，无需额外依赖)
- **AAC** (需要 `decoder-ffi` feature，通过 FFI 对接 C++/FFmpeg)
- **MP3** (需要 `decoder-ffi` feature，通过 FFI 对接 C++/FFmpeg)
- **Opus** (需要 `decoder-ffi` feature，通过 FFI 对接 C++/FFmpeg)

## 快速开始

### 1. 解码 G.711 音频（纯 Rust，推荐）

G.711 解码器使用纯 Rust 实现，无需任何外部依赖，适合快速集成。

```rust
use bytes::Bytes;
use rtp::decoder::{
    create_decoder, CodecType, MediaPacket,
    SampleFormat, MediaFrame,
};

// 创建 G.711 A-law 解码器
let mut decoder = create_decoder(CodecType::G711A).unwrap();

// 准备编码数据（假设从 RTP 包中获取）
let encoded_data = Bytes::from(vec![0xD5; 160]); // 160 字节 = 20ms @ 8kHz
let packet = MediaPacket::new(CodecType::G711A, encoded_data)
    .with_timestamps(0, 0);

// 解码
let result = decoder.decode(&packet).unwrap();
if let Some(frame) = result {
    // 解码成功，获取 PCM 数据
    assert!(frame.is_audio());
    assert_eq!(frame.sample_format, SampleFormat::S16); // 输出 S16 PCM
    assert_eq!(frame.sample_rate, 8000); // G.711 默认 8kHz
    assert_eq!(frame.nb_samples, 160); // 160 个采样点
    
    // 获取 PCM 数据
    let pcm_data: &[u8] = frame.as_slice();
    println!("解码成功，PCM 数据大小: {} 字节", pcm_data.len());
}
```

### 2. 解码 AAC 音频（需要 FFI）

AAC 解码器需要启用 `decoder-ffi` feature，并依赖 C++/FFmpeg 后端。

```toml
# Cargo.toml
[features]
decoder-ffi = ["dep:some-ffi-dep"]  # 根据实际依赖配置
```

```rust
use rtp::decoder::{create_decoder, CodecType};

// 创建 AAC 解码器
let mut decoder = create_decoder(CodecType::AAC).unwrap();

// 解码使用方式与 G.711 相同
// ...
```

## 核心概念

### SampleFormat（采样格式）

`SampleFormat` 枚举定义了音频采样格式，对齐 C++ 端和 FFmpeg 的 `AVSampleFormat`：

```rust
pub enum SampleFormat {
    Unknown,  // 未知格式
    U8,       // 8-bit unsigned integer
    S16,      // 16-bit signed integer
    S32,      // 32-bit signed integer
    F32,      // 32-bit floating point
    F64,      // 64-bit floating point
    U8P,      // 8-bit unsigned integer (planar)
    S16P,     // 16-bit signed integer (planar)
    S32P,     // 32-bit signed integer (planar)
    F32P,     // 32-bit floating point (planar)
    F64P,     // 64-bit floating point (planar)
}
```

**关键方法**：

- `is_planar()` - 是否为 planar 格式（平面存储）
- `is_packed()` - 是否为 packed 格式（交错存储）
- `bytes_per_sample()` - 每个采样点的字节数
- `to_planar()` / `to_packed()` - 格式转换

### MediaFrame（解码后帧）

`MediaFrame` 结构体同时支持视频帧和音频帧。对于音频帧，以下字段有效：

```rust
pub struct MediaFrame {
    // 媒体类型（Audio/Video）
    pub media_type: MediaType,
    
    // 音频相关字段
    pub sample_format: SampleFormat,  // 采样格式
    pub sample_rate: i32,            // 采样率（Hz）
    pub channels: i32,               // 声道数
    pub channel_layout: u64,         // 声道布局（bitmask）
    pub nb_samples: i32,             // 每声道的采样点数
    pub bytes_per_sample: i32,       // 每采样字节数
    pub planar: bool,                // 是否为 planar 格式
    
    // 通用字段
    pub pts: i64,                    // 显示时间戳（微秒）
    pub dts: i64,                    // 解码时间戳（微秒）
    pub duration: i64,               // 帧持续时间（微秒）
    pub data: Bytes,                 // 帧数据（PCM）
    // ...
}
```

**音频辅助方法**：

```rust
impl MediaFrame {
    // 创建音频帧
    pub fn new_audio(
        sample_format: SampleFormat,
        sample_rate: i32,
        channels: i32,
        nb_samples: i32,
        data: Bytes,
    ) -> Self;
    
    // 是否为音频帧
    pub fn is_audio(&self) -> bool;
    
    // 是否为视频帧
    pub fn is_video(&self) -> bool;
    
    // 获取音频帧的字节大小
    pub fn audio_frame_size(&self) -> usize;
    
    // 获取指定声道的音频数据（仅适用于 planar 格式）
    pub fn audio_channel_data(&self, channel_index: usize) -> Option<&[u8]>;
    
    // 计算音频帧的持续时间（秒）
    pub fn audio_duration_seconds(&self) -> f64;
}
```

## 使用示例

### 示例 1：批量解码 G.711 音频

```rust
use bytes::Bytes;
use rtp::decoder::{create_decoder, CodecType, MediaPacket};

fn decode_g711_stream(encoded_packets: Vec<Vec<u8>>) {
    let mut decoder = create_decoder(CodecType::G711A).unwrap();
    
    for (i, encoded_data) in encoded_packets.iter().enumerate() {
        let packet = MediaPacket::new(
            CodecType::G711A,
            Bytes::from(encoded_data.clone()),
        ).with_timestamps(i as i64 * 20000, i as i64 * 20000); // 20ms per packet
        
        let result = decoder.decode(&packet).unwrap();
        if let Some(frame) = result {
            // 处理解码后的 PCM 数据
            let pcm_data = frame.as_slice();
            println!("帧 {}: {} 字节 PCM 数据", i, pcm_data.len());
            
            // 计算持续时间
            let duration = frame.audio_duration_seconds();
            println!("持续时间: {:.3} 秒", duration);
        }
    }
}
```

### 示例 2：处理 Planar 格式的音频

```rust
use rtp::decoder::{create_decoder, CodecType, SampleFormat};

fn process_planar_audio() {
    // 假设使用 Opus 解码器（输出 F32P 格式）
    let mut decoder = create_decoder(CodecType::Opus).unwrap();
    
    // ... 解码过程 ...
    
    // 假设解码后得到 planar 格式的帧
    let frame: MediaFrame = ...;
    
    if frame.planar {
        // 分别处理每个声道
        for ch in 0..frame.channels as usize {
            if let Some(channel_data) = frame.audio_channel_data(ch) {
                println!("声道 {}: {} 字节", ch, channel_data.len());
                // 处理声道数据...
            }
        }
    } else {
        // packed 格式：所有声道交错存储
        let interleaved_data = frame.as_slice();
        println!("交错数据: {} 字节", interleaved_data.len());
    }
}
```

### 示例 3：转换 SampleFormat

```rust
use rtp::decoder::SampleFormat;

fn convert_sample_format() {
    let format = SampleFormat::S16;
    
    // 转换为 planar 格式
    if let Some(planar_format) = format.to_planar() {
        println!("S16 的 planar 格式是: {:?}", planar_format); // S16P
    }
    
    // 转换为 packed 格式
    let planar = SampleFormat::F32P;
    if let Some(packed_format) = planar.to_packed() {
        println!("F32P 的 packed 格式是: {:?}", packed_format); // F32
    }
    
    // 检查格式属性
    println!("S16 是否为 planar: {}", SampleFormat::S16.is_planar()); // false
    println!("F32P 是否为 planar: {}", SampleFormat::F32P.is_planar()); // true
    println!("S16 每采样字节数: {}", SampleFormat::S16.bytes_per_sample()); // 2
    println!("F32 每采样字节数: {}", SampleFormat::F32.bytes_per_sample()); // 4
}
```

## API 参考

### 创建解码器

```rust
/// 创建指定编解码类型的解码器
pub fn create_decoder(codec: CodecType) -> DecodeResult<Box<dyn Decoder + Send>>;

/// 创建音频解码器（直接调用）
pub fn create_audio_decoder(codec_type: CodecType) -> DecodeResult<Box<dyn Decoder + Send>>;

/// 检查是否为支持的音频编解码器
pub fn is_supported_audio_codec(codec_type: CodecType) -> bool;
```

### Decoder Trait

所有解码器都实现 `Decoder` trait：

```rust
pub trait Decoder {
    /// 解码数据包
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>;
    
    /// 刷新解码器（获取缓存的帧）
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>>;
    
    /// 重置解码器状态
    fn reset(&mut self);
    
    /// 获取编解码器类型
    fn codec_type(&self) -> CodecType;
    
    /// 获取解码器名称
    fn name(&self) -> &str;
}
```

### 检查支持的解码器

```rust
use rtp::decoder::{is_codec_supported, supported_codecs, CodecType};

// 检查特定编解码器是否支持
assert!(is_codec_supported(CodecType::G711A)); // true（纯 Rust）
assert!(is_codec_supported(CodecType::AAC)); // 取决于 feature

// 获取所有支持的解码器
let codecs = supported_codecs();
println!("支持的解码器: {:?}", codecs);
```

## G.711 解码器详细说明

### G.711 A-law 解码器

```rust
pub struct G711ADecoder {
    sample_rate: i32,  // 采样率（默认 8000 Hz）
    channels: i32,     // 声道数（默认 1）
}

impl G711ADecoder {
    pub fn new() -> DecodeResult<Self>;
    pub fn with_sample_rate(mut self, sample_rate: i32) -> Self;
    pub fn with_channels(mut self, channels: i32) -> Self;
}
```

**特性**：
- 输入：8-bit A-law 编码数据
- 输出：16-bit S16 PCM（packed，单声道）
- 默认采样率：8000 Hz
- 无状态解码器，无需 flush/reset

### G.711 μ-law 解码器

```rust
pub struct G711UDecoder {
    sample_rate: i32,  // 采样率（默认 8000 Hz）
    channels: i32,     // 声道数（默认 1）
}

impl G711UDecoder {
    pub fn new() -> DecodeResult<Self>;
    pub fn with_sample_rate(mut self, sample_rate: i32) -> Self;
    pub fn with_channels(mut self, channels: i32) -> Self;
}
```

**特性**：
- 输入：8-bit μ-law 编码数据
- 输出：16-bit S16 PCM（packed，单声道）
- 默认采样率：8000 Hz
- 无状态解码器，无需 flush/reset

## FFI 接口（高级）

如果需要直接对接 C++/FFmpeg 后端，可以使用 FFI 接口：

```rust
use rtp::decoder::ffi::{FfiDecoder, DecodedFrame};

// 创建 FFI 解码器
let mut decoder = FfiDecoder::new_audio(
    CodecType::AAC.to_u32() as i32,
    SampleFormat::S16.to_u32() as i32,
).unwrap();

// 解码数据
decoder.decode(data, pts, dts, keyframe).unwrap();

// 获取解码后的帧
let frame = decoder.get_frame().unwrap();
if let Some(decoded_frame) = frame {
    // 转换为 MediaFrame
    let media_frame = decoded_frame.to_media_frame(data_bytes);
}
```

**注意**：FFI 接口需要启用 `decoder-ffi` feature，并且需要 C++ 后端库支持。

## 性能优化建议

1. **使用 G.711 纯 Rust 解码器**：无需 FFI 调用开销，延迟更低
2. **批量处理**：一次性解码多个数据包，减少函数调用开销
3. **避免不必要的数据拷贝**：直接使用 `MediaFrame.data` 的引用
4. **Planar 到 Packed 转换**：如果需要 interleaved 格式，提前转换

## 常见问题

### Q1: 为什么 G.711 解码器输出是 S16 格式？

G.711 标准定义了解码到 16-bit PCM。如果需要其他格式，可以在应用层进行转换。

### Q2: 如何支持多声道音频？

G.711 通常用于单声道（电话系统）。如果需要多声道，可以创建多个解码器实例，每个声道一个。

### Q3: AAC 解码器需要哪些依赖？

AAC 解码器需要启用 `decoder-ffi` feature，并链接到 C++/FFmpeg 库。具体配置取决于项目设置。

### Q4: 如何处理解码错误？

```rust
match decoder.decode(&packet) {
    Ok(Some(frame)) => {
        // 解码成功
    }
    Ok(None) => {
        // 需要更多数据
    }
    Err(e) => {
        // 解码错误，处理错误
        eprintln!("解码错误: {:?}", e);
    }
}
```

## 测试

运行单元测试：

```bash
cargo test --package your-package -- rtp::decoder
```

测试覆盖：
- SampleFormat 转换
- MediaFrame 音频字段
- G.711 解码器
- 解码器工厂

## 参考

- [G.711 标准](https://en.wikipedia.org/wiki/G.711)
- [FFmpeg AVSampleFormat](https://ffmpeg.org/doxygen/trunk/group__lavu__sampfmts.html)
- [RTP Payload Format for G.711](https://tools.ietf.org/html/rfc3551)
