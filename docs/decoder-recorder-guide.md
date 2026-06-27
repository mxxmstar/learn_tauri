# 解码器与录像模块完整指导说明

## 目录

1. [概述](#1-概述)
2. [模块架构](#2-模块架构)
3. [Feature Flags 配置](#3-feature-flags-配置)
4. [解码器模块使用指南](#4-解码器模块使用指南)
5. [录像模块使用指南](#5-录像模块使用指南)
6. [FFI 接口对接指南](#6-ffi-接口对接指南)
7. [代码审查问题修复记录](#7-代码审查问题修复记录)
8. [测试指南](#8-测试指南)
9. [常见问题](#9-常见问题)

---

## 1. 概述

本项目实现了 RTP/AVTP 流的解码与录像功能，包含两大核心模块：

- **解码器模块** (`rtp::decoder`)：支持 H.264、H.265、MJPEG 视频解码和 G.711、AAC、MP3、Opus 音频解码
- **录像模块** (`recorder`)：将解码后的视频流封装为 MP4 (H.264/H.265) 或 AVI (MJPEG) 文件

### 设计原则

- **条件编译**：通过 Feature Flags 控制功能模块，纯 Rust 实现始终可用，FFI 实现按需启用
- **统一接口**：所有解码器实现 `Decoder` trait，所有录像器实现 `Recorder` trait
- **零拷贝**：使用 `bytes::Bytes` 避免不必要的数据拷贝
- **线程安全**：所有解码器和录像器实现 `Send`，支持跨线程使用

---

## 2. 模块架构

### 2.1 目录结构

```
src-tauri/src/
├── rtp/
│   └── decoder/
│       ├── mod.rs              # 模块入口，导出公共类型和工厂函数
│       ├── types.rs            # 枚举定义（CodecType, PixelFormat, SampleFormat 等）
│       ├── frame.rs            # MediaPacket 和 MediaFrame 定义
│       ├── trait_.rs           # Decoder trait 定义和 DecodeError
│       ├── ffi.rs              # FFI 解码器接口（对接 C++/FFmpeg）
│       ├── h264.rs             # H.264 解码器（需要 decoder-ffi）
│       ├── h265.rs             # H.265 解码器（需要 decoder-ffi）
│       ├── mjpeg.rs            # MJPEG 解码器（纯 Rust，需要 decoder-rust）
│       └── audio/
│           ├── mod.rs          # 音频解码器模块入口
│           ├── g711.rs         # G.711 A-law/μ-law 解码器（纯 Rust）
│           ├── aac.rs          # AAC 解码器（需要 decoder-ffi）
│           ├── mp3.rs          # MP3 解码器（需要 decoder-ffi）
│           └── opus.rs         # Opus 解码器（需要 decoder-ffi）
├── recorder/
│   ├── mod.rs                  # 模块入口，工厂函数
│   ├── config.rs               # RecorderConfig 和 ContainerFormat
│   ├── error.rs                # RecordError 和 RecordResult
│   ├── trait_.rs               # Recorder trait 和 RecordStats
│   └── ffi.rs                  # FFI 录像器接口（对接 C++/FFmpeg）
└── bin/
    └── recorder_example.rs     # 录像模块示例程序
```

### 2.2 数据流

```
RTP/AVTP 数据包
    │
    ▼
RTP 解析器 / AVTP 解析器
    │
    ▼
MediaPacket（编码数据包）
    │
    ▼
Decoder::decode() ──→ MediaFrame（解码后帧）
    │                       │
    │                       ▼
    │              Recorder::write_media_frame()
    │                       │
    ▼                       ▼
显示/渲染               FFI → C++/FFmpeg → 视频文件
```

---

## 3. Feature Flags 配置

### 3.1 Cargo.toml 配置

```toml
[features]
default = ["decoder-rust"]
decoder-rust = []   # 纯 Rust 解码器（MJPEG）
decoder-ffi = []    # FFI 解码器（H264/H265/AAC/MP3/Opus）
recorder-ffi = []   # FFI 录像器（MP4/AVI）
recorder-mp4 = ["recorder-ffi"]  # MP4 录像器（便捷别名）
recorder-avi = ["recorder-ffi"]  # AVI 录像器（便捷别名）
clap = ["dep:clap"]  # 启用 clap 依赖（用于示例程序）
```

### 3.2 功能矩阵

| Feature | 解码器 | 录像器 | 依赖 |
|---------|--------|--------|------|
| `decoder-rust` (默认) | MJPEG | - | 无 |
| `decoder-ffi` | H.264, H.265, AAC, MP3, Opus | - | C++/FFmpeg 库 |
| `recorder-ffi` | - | MP4, AVI | C++/FFmpeg 库 |

### 3.3 编译命令

```bash
# 默认编译（仅 MJPEG 解码）
cargo build

# 启用所有解码器
cargo build --features decoder-ffi

# 启用录像功能
cargo build --features recorder-ffi

# 启用全部功能
cargo build --features "decoder-ffi recorder-ffi"

# 编译示例程序
cargo build --bin recorder_example --features "recorder-ffi clap"
```

---

## 4. 解码器模块使用指南

### 4.1 核心类型

#### CodecType（编解码器类型）

```rust
pub enum CodecType {
    H264, H265, MJPEG,           // 视频
    MPEG4, VP8, VP9, AV1,        // 视频（预留）
    AAC, MP3, OPUS,              // 音频
    G711A, G711U,                // 音频（G.711 A-law / μ-law）
    Unknown,
}
```

#### MediaPacket（编码数据包，解码器输入）

```rust
pub struct MediaPacket {
    pub media_type: MediaType,    // Video / Audio
    pub codec_type: CodecType,    // 编解码器类型
    pub pts: i64,                 // 显示时间戳（微秒）
    pub dts: i64,                 // 解码时间戳（微秒）
    pub keyframe: bool,           // 是否为关键帧
    pub data: Bytes,              // 编码数据
    pub backend: Option<BackendHandle>,
}
```

#### MediaFrame（解码后帧，解码器输出）

```rust
pub struct MediaFrame {
    pub media_type: MediaType,
    // 视频字段
    pub pixel_format: PixelFormat,
    pub width: i32, pub height: i32,
    pub stride: [i32; 8],
    pub plane_offset: [i32; 8],
    pub plane_count: i32,
    // 音频字段
    pub sample_format: SampleFormat,
    pub sample_rate: i32, pub channels: i32,
    pub nb_samples: i32, pub bytes_per_sample: i32,
    pub planar: bool,
    // 通用字段
    pub pts: i64, pub dts: i64, pub duration: i64,
    pub keyframe: bool,
    pub data: Bytes,
}
```

#### Decoder Trait

```rust
pub trait Decoder: Send {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>;
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>>;
    fn reset(&mut self);
    fn codec_type(&self) -> CodecType;
    fn name(&self) -> &str;
}
```

### 4.2 创建解码器

#### 方式一：使用工厂函数（推荐）

```rust
use rtp::decoder::{create_decoder, CodecType};

// 创建 MJPEG 解码器（纯 Rust，始终可用）
let mut decoder = create_decoder(CodecType::MJPEG)?;

// 创建 H.264 解码器（需要 decoder-ffi feature）
let mut decoder = create_decoder(CodecType::H264)?;

// 创建 G.711 A-law 解码器（纯 Rust，始终可用）
let mut decoder = create_decoder(CodecType::G711A)?;
```

#### 方式二：使用配置对象

```rust
use rtp::decoder::{DecoderConfig, CodecType, PixelFormat};

let config = DecoderConfig::new(CodecType::MJPEG)
    .with_output_format(PixelFormat::RGBA)
    .with_stats(true);  // 启用统计信息

let mut decoder = config.create_decoder()?;
```

### 4.3 解码视频帧

```rust
use bytes::Bytes;
use rtp::decoder::{create_decoder, CodecType, MediaPacket};

let mut decoder = create_decoder(CodecType::MJPEG)?;

// 从 RTP 包获取编码数据
let encoded_data = Bytes::from(vec![0xFF, 0xD8, /* ... JPEG 数据 ... */ 0xFF, 0xD9]);

// 创建 MediaPacket
let packet = MediaPacket::new(CodecType::MJPEG, encoded_data)
    .with_timestamps(pts_us, dts_us)  // 微秒
    .with_keyframe(true);

// 解码
match decoder.decode(&packet)? {
    Some(frame) => {
        // frame.data 包含解码后的像素数据
        // frame.pixel_format 指示像素格式（如 RGBA）
        // frame.width, frame.height 为图像尺寸
        println!("解码成功: {}x{} {:?}", frame.width, frame.height, frame.pixel_format);
    }
    None => {
        // 需要更多数据（如 H.264 需要多个 NAL 单元）
    }
}
```

### 4.4 解码音频帧

#### G.711 解码（纯 Rust）

```rust
use bytes::Bytes;
use rtp::decoder::{create_decoder, CodecType, MediaPacket, SampleFormat};

let mut decoder = create_decoder(CodecType::G711A)?;

// G.711 A-law 编码数据（8-bit 压缩）
let encoded_data = Bytes::from(vec![0xD5; 160]); // 160 字节 = 20ms @ 8kHz

let packet = MediaPacket::new(CodecType::G711A, encoded_data)
    .with_timestamps(0, 0);

let frame = decoder.decode(&packet)?.unwrap();

// G.711 解码输出
assert!(frame.is_audio());
assert_eq!(frame.sample_format, SampleFormat::S16);  // 16-bit PCM
assert_eq!(frame.sample_rate, 8000);                  // 8kHz
assert_eq!(frame.nb_samples, 160);                    // 160 个采样点
// data 大小 = 160 * 2 = 320 字节（S16 = 2 bytes/sample）
```

#### G.711 解码器自定义配置

```rust
use rtp::decoder::G711ADecoder;

// 自定义采样率和声道数
let mut decoder = G711ADecoder::new()?
    .with_sample_rate(16000)  // 16kHz
    .with_channels(2);        // 立体声
```

### 4.5 检查支持的编解码器

```rust
use rtp::decoder::{is_codec_supported, supported_codecs, CodecType};

// 检查特定编解码器
assert!(is_codec_supported(CodecType::MJPEG));     // 始终支持
assert!(is_codec_supported(CodecType::G711A));     // 始终支持
// assert!(is_codec_supported(CodecType::H264));   // 需要 decoder-ffi

// 获取所有支持的编解码器
let codecs = supported_codecs();
for codec in &codecs {
    println!("支持: {:?}", codec);
}
```

### 4.6 统计信息

```rust
use rtp::decoder::{DecoderConfig, StatsDecoder, CodecType};

let config = DecoderConfig::new(CodecType::MJPEG).with_stats(true);
let mut decoder = config.create_decoder()?;

// 解码若干帧后获取统计信息
// （StatsDecoder 自动统计，通过 downcast 获取）
```

---

## 5. 录像模块使用指南

### 5.1 核心类型

#### RecorderConfig（录像配置）

```rust
pub struct RecorderConfig {
    pub codec_type: CodecType,                    // 编解码器类型
    pub container_format: Option<ContainerFormat>, // 容器格式（None=自动选择）
    pub output_path: PathBuf,                     // 输出文件路径
    pub width: Option<u32>,                       // 视频宽度
    pub height: Option<u32>,                      // 视频高度
    pub framerate: Option<f64>,                   // 帧率
    pub enable_timestamp: bool,                   // 是否启用时间戳记录
}
```

#### ContainerFormat（容器格式）

```rust
pub enum ContainerFormat {
    MP4,  // 用于 H.264/H.265
    AVI,  // 用于 MJPEG
}
```

容器格式根据编解码器类型自动选择：
- H.264 / H.265 → MP4
- MJPEG → AVI

#### Recorder Trait

```rust
pub trait Recorder {
    fn start(&mut self, config: RecorderConfig) -> RecordResult<()>;
    fn write_frame(&mut self, frame: &[u8], timestamp_ms: Option<u64>) -> RecordResult<()>;
    fn write_media_frame(&mut self, frame: &MediaFrame) -> RecordResult<()>;
    fn finish(&mut self) -> RecordResult<()>;
    fn get_stats(&self) -> RecordStats;
    fn is_recording(&self) -> bool;
    fn get_config(&self) -> Option<&RecorderConfig>;
    fn cancel(&mut self) -> RecordResult<()>;
}
```

### 5.2 基本使用流程

```rust
use std::path::PathBuf;
use recorder::{create_recorder, Recorder, RecorderConfig};
use rtp::decoder::CodecType;

// 1. 创建配置
let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"))
    .with_dimensions(1920, 1080)
    .with_framerate(30.0);

// 2. 创建录像器
let mut recorder = create_recorder(&config)?;

// 3. 开始录像
recorder.start(config.clone())?;

// 4. 写入视频帧
for i in 0..100 {
    let frame_data = vec![0u8; 1024];  // 模拟帧数据
    let timestamp_ms = Some((i as f64 * 1000.0 / 30.0) as u64);
    recorder.write_frame(&frame_data, timestamp_ms)?;
}

// 5. 结束录像
recorder.finish()?;

// 6. 查看统计信息
let stats = recorder.get_stats();
println!("写入帧数: {}", stats.frames_written);
println!("写入字节: {}", stats.bytes_written);
println!("持续时间: {:?} ms", stats.duration_ms);
```

### 5.3 写入 MediaFrame

如果已有解码后的 `MediaFrame`，可以直接写入：

```rust
// write_media_frame 会自动提取时间戳和关键帧信息
recorder.write_media_frame(&decoded_frame)?;
```

### 5.4 取消录像

```rust
// 取消录像（不保存文件）
recorder.cancel()?;
```

### 5.5 容器格式选择

```rust
// 自动选择（根据编解码器类型）
let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"));
// config.get_container_format() == Some(ContainerFormat::MP4)

// 手动指定
let config = RecorderConfig::new(CodecType::MJPEG, PathBuf::from("output.avi"))
    .with_container_format(ContainerFormat::AVI);
```

### 5.6 配置验证

```rust
let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"));

// 验证配置有效性
match config.validate() {
    Ok(()) => println!("配置有效"),
    Err(e) => println!("配置无效: {}", e),
}
```

验证规则：
- 容器格式必须支持该编解码器（MP4 → H.264/H.265，AVI → MJPEG）
- 输出路径的父目录必须存在

---

## 6. FFI 接口对接指南

### 6.1 解码器 FFI 接口

C++ 端需要实现以下 C 风格接口：

```cpp
// 创建解码器
// codec_type: 见 CodecType::to_u32() 映射
// pixel_format: 见 PixelFormat::to_u32() 映射（视频）
// sample_format: 见 SampleFormat::to_u32() 映射（音频）
// media_type: 0=Video, 1=Audio
DecoderHandle decoder_create(int codec_type, int format, int media_type);

// 销毁解码器
void decoder_destroy(DecoderHandle handle);

// 解码一帧数据
int decoder_decode(DecoderHandle handle, const uint8_t* data, size_t size,
                   int64_t pts, int64_t dts, bool keyframe);

// 获取解码后的帧
int decoder_get_frame(DecoderHandle handle, DecodedFrame* frame);

// 刷新解码器
int decoder_flush(DecoderHandle handle);

// 重置解码器
void decoder_reset(DecoderHandle handle);
```

### 6.2 录像器 FFI 接口

```cpp
// 创建录像器
RecorderHandle recorder_create(int codec_type, int container_format,
                               const char* output_path);

// 销毁录像器
void recorder_destroy(RecorderHandle handle);

// 开始录像
int recorder_start(RecorderHandle handle, int width, int height, double framerate);

// 写入视频帧
int recorder_write_frame(RecorderHandle handle, const uint8_t* data, size_t size,
                         int64_t timestamp_ms, int keyframe);

// 结束录像
int recorder_finish(RecorderHandle handle);

// 取消录像
int recorder_cancel(RecorderHandle handle);

// 获取统计信息
int recorder_get_stats(RecorderHandle handle, RecorderStats* stats);
```

### 6.3 C++ 端 DecodedFrame 结构

```cpp
struct DecodedFrame {
    int32_t media_type;      // 0=Video, 1=Audio
    int32_t codec_type;

    // 视频字段
    int32_t pixel_format;
    int32_t width, height;
    int32_t stride[8];
    int32_t plane_offset[8];
    int32_t plane_count;

    // 音频字段
    int32_t sample_format;
    int32_t sample_rate;
    int32_t channels;
    uint64_t channel_layout;
    int32_t nb_samples;
    int32_t bytes_per_sample;
    bool planar;

    // 数据字段
    const uint8_t* data;
    size_t size;
    int64_t pts, dts;
    int64_t duration;
    bool keyframe;
};
```

### 6.4 线程安全说明

- `FfiDecoder` 和 `FfiRecorder` 通过 `unsafe impl Send` 声明为可跨线程传递
- **前提条件**：C++ 端实现必须保证同一实例不会被多线程同时访问
- **建议**：每个线程使用独立的解码器/录像器实例，或在 Rust 端使用 `Mutex` 包装

---

## 7. 代码审查问题修复记录

本次代码审查发现并修复了以下问题：

### 7.1 致命问题（编译错误）

| # | 文件 | 问题 | 修复方案 |
|---|------|------|----------|
| 1 | `Cargo.toml` | `clap` 依赖放在 `[features]` 表中导致 TOML 解析错误 | 移至 `[dependencies]` 表，设为 `optional = true` |
| 2 | `decoder/ffi.rs` | `decode`/`get_frame`/`flush`/`reset` 方法定义在 `impl` 块外部 | 将方法移入 `impl FfiDecoder` 块内 |
| 3 | `decoder/ffi.rs` | 测试调用不存在的 `FfiDecoder::new()` | 改为 `FfiDecoder::new_video()` |
| 4 | `decoder/h264.rs` `h265.rs` | 使用 `trait` 关键字作为模块名 | 改为 `trait_` |
| 5 | `audio/aac.rs` `mp3.rs` `opus.rs` | `Decoder` trait 签名不匹配（返回类型、缺少 `name()`、多余 `media_type()`） | 统一为 `DecodeResult` 返回类型，添加 `name()`，移除 `media_type()` |
| 6 | `audio/aac.rs` `mp3.rs` `opus.rs` | `FfiDecoder` 包含裸指针不满足 `Send` | 添加 `unsafe impl Send for FfiDecoder` |
| 7 | `audio/mod.rs` | `AacDecoder::new()` 返回 `Result<_, String>` 无法用 `?` 转为 `DecodeError` | 为 `DecodeError` 添加 `From<String>` 实现 |
| 8 | `bin/recorder_example.rs` | 引用不存在的 `recorder::mp4::Mp4Recorder` | 重写为使用 `create_recorder()` 工厂函数 |
| 9 | `bin/recorder_example.rs` | 使用 clap v3 API（`App::new`） | 升级为 clap v4 API（`Command::new`） |
| 10 | `bin/recorder_example.rs` | 导入 `recorder`/`rtp` 为外部 crate | 改为 `learn_tauri_lib::recorder`/`learn_tauri_lib::rtp` |

### 7.2 算法错误

| # | 文件 | 问题 | 修复方案 |
|---|------|------|----------|
| 11 | `audio/g711.rs` | A-law 解码使用 `!alaw`（按位取反）而非 `alaw ^ 0x55`（异或） | 改为标准 G.711 算法 `alaw ^ 0x55` |
| 12 | `audio/g711.rs` | μ-law 解码公式不正确 | 改为标准公式 `((mantissa << 3) + 0x84) << exponent - 0x84` |
| 13 | `audio/g711.rs` | 测试期望值错误 | 更新测试为正确的 G.711 标准值 |

### 7.3 逻辑/设计问题

| # | 文件 | 问题 | 修复方案 |
|---|------|------|----------|
| 14 | `decoder/mod.rs` | `create_decoder` 未路由 MP3/Opus | 添加 `CodecType::MP3 \| CodecType::OPUS` 分支 |
| 15 | `decoder/mod.rs` | `is_codec_supported`/`supported_codecs` 缺少 MP3/Opus | 补充对应分支 |
| 16 | `audio/mod.rs` | `is_supported_audio_codec` 缺少 MP3/Opus | 补充匹配 |
| 17 | `audio/mp3.rs` | `codec_type()` 返回 `Unknown` | 改为 `CodecType::MP3` |
| 18 | `audio/opus.rs` | `codec_type()` 返回 `Unknown` | 改为 `CodecType::OPUS` |
| 19 | `recorder/mod.rs` | `create_recorder` 内部调用 `start()` 导致用户再调 `start()` 失败 | 移除内部 `start()` 调用，由用户显式调用 |
| 20 | `recorder/ffi.rs` | `FfiRecorder` 不存储 `RecorderConfig` | 添加 `config` 字段，`start()` 时存储 |
| 21 | `recorder/trait_.rs` | `write_media_frame` 将 `pts == 0` 视为无时间戳 | 改为 `pts >= 0` |

### 7.4 G.711 算法修复详情

#### 修复前（错误）

```rust
// A-law：使用按位取反（错误！应为 XOR 0x55）
let alaw = !alaw;

// μ-law：公式不正确
let pcm = ((mantissa << 1) | 0x21) << (exponent + 1);
```

#### 修复后（正确，符合 ITU-T G.711 标准）

```rust
// A-law：XOR 0x55 还原交替位翻转
let alaw = alaw ^ 0x55;
let mut pcm: i16 = ((mantissa as i16) << 4) + 8;
if exponent != 0 {
    pcm += 0x100;
    pcm <<= exponent - 1;
}

// μ-law：标准对数扩展公式
let ulaw = !ulaw;
let mut pcm: i16 = ((mantissa as i16) << 3) + 0x84;
pcm <<= exponent;
pcm -= 0x84;
```

#### 验证结果

- A-law `0xD5`（静音）→ 解码为 `8`（修复前为 `31612`）
- A-law `0x55`（负静音）→ 解码为 `-8`（修复前为 `-31612`）
- μ-law `0xFF`（静音）→ 解码为 `0`（修复前为 `66`）
- μ-law `0x7F`（静音）→ 解码为 `0`（修复前为 `-66`）

---

## 8. 测试指南

### 8.1 运行测试

```bash
# 运行所有测试
cargo test --lib

# 运行解码器测试
cargo test --lib rtp::decoder

# 运行 G.711 测试
cargo test --lib rtp::decoder::audio::g711

# 运行录像模块测试
cargo test --lib recorder

# 带 FFI feature 运行测试
cargo test --lib --features decoder-ffi
```

### 8.2 测试覆盖

| 模块 | 测试项 | 说明 |
|------|--------|------|
| `types.rs` | `test_media_type_conversion` | MediaType u32 转换 |
| `types.rs` | `test_codec_type` | CodecType 转换和分类 |
| `types.rs` | `test_pixel_format` | PixelFormat 属性和帧大小计算 |
| `types.rs` | `test_sample_format_*` | SampleFormat 转换、planar/packed、字节数 |
| `frame.rs` | `test_media_packet` | MediaPacket 创建和时间戳 |
| `frame.rs` | `test_media_frame_*` | MediaFrame 视频帧/音频帧创建 |
| `g711.rs` | `test_g711a_decoder` | G.711 A-law 解码输出格式 |
| `g711.rs` | `test_g711u_decoder` | G.711 μ-law 解码输出格式 |
| `g711.rs` | `test_alaw_decode` | A-law 静音解码值验证 |
| `g711.rs` | `test_ulaw_decode` | μ-law 静音解码值验证 |
| `trait_.rs` | `test_decoder_info` | DecoderInfo 创建 |
| `trait_.rs` | `test_stats_decoder` | StatsDecoder 统计功能 |
| `mod.rs` | `test_create_*` | 解码器工厂函数 |
| `recorder/` | `test_*` | 容器格式、配置验证、统计信息 |

### 8.3 G.711 测试验证

```rust
#[test]
fn test_alaw_decode() {
    // 0xD5 是 A-law 编码的静音（正方向），解码为 8
    let pcm_pos = alaw_to_pcm(0xD5);
    // 0x55 是 A-law 编码的静音（负方向），解码为 -8
    let pcm_neg = alaw_to_pcm(0x55);
    assert_eq!(pcm_pos, 8);
    assert_eq!(pcm_neg, -8);
    assert_eq!(pcm_pos, -pcm_neg); // 互为相反数
}

#[test]
fn test_ulaw_decode() {
    // 0xFF 和 0x7F 是 μ-law 编码的静音，都解码为 0
    let pcm1 = ulaw_to_pcm(0xFF);
    let pcm2 = ulaw_to_pcm(0x7F);
    assert_eq!(pcm1, pcm2);
    assert_eq!(pcm1, 0);
}
```

---

## 9. 常见问题

### Q1: 为什么默认只支持 MJPEG 解码？

默认启用 `decoder-rust` feature，仅包含纯 Rust 实现的 MJPEG 解码器。H.264/H.265/AAC/MP3/Opus 需要通过 FFI 对接 C++/FFmpeg，需启用 `decoder-ffi` feature。

### Q2: 如何启用 FFI 解码器？

1. 确保 C++/FFmpeg 库已编译并可供链接
2. 在 `Cargo.toml` 中启用 `decoder-ffi` feature
3. 编译时添加 `--features decoder-ffi`

### Q3: G.711 解码器为什么输出 S16 格式？

G.711 标准定义了 8-bit 压缩到 16-bit PCM 的解码。输出的 S16 (16-bit signed integer) 是标准的 PCM 格式，可直接用于音频播放或进一步处理。

### Q4: 录像模块支持哪些格式？

| 编解码器 | 容器格式 | 说明 |
|----------|----------|------|
| H.264 | MP4 | 通过 FFmpeg libavformat 封装 |
| H.265 | MP4 | 通过 FFmpeg libavformat 封装 |
| MJPEG | AVI | 通过 FFmpeg libavformat 封装 |

### Q5: 录像器的 `write_frame` 和 `write_media_frame` 有什么区别？

- `write_frame(&[u8], Option<u64>)`：低级接口，只接收原始字节和时间戳，无法传递关键帧信息
- `write_media_frame(&MediaFrame)`：高级接口，自动提取 PTS、关键帧标记等信息

建议优先使用 `write_media_frame`。

### Q6: FfiDecoder/FfiRecorder 为什么需要 `unsafe impl Send`？

因为它们包含 C++ 端的裸指针（`*mut c_void`），Rust 默认不为包含裸指针的类型实现 `Send`。通过 `unsafe impl Send` 声明可以跨线程传递，前提是 C++ 端实现保证线程安全（或不在多线程同时访问同一实例）。

### Q7: 如何自定义 G.711 解码器的采样率？

```rust
use rtp::decoder::G711ADecoder;

let decoder = G711ADecoder::new()?
    .with_sample_rate(16000)  // 16kHz（默认 8kHz）
    .with_channels(2);        // 立体声（默认单声道）
```

### Q8: 如何处理解码错误？

```rust
match decoder.decode(&packet) {
    Ok(Some(frame)) => {
        // 解码成功，处理帧
    }
    Ok(None) => {
        // 需要更多数据（如 H.264 等待完整帧）
    }
    Err(DecodeError::DecodeFailed(msg)) => {
        eprintln!("解码失败: {}", msg);
    }
    Err(DecodeError::UnsupportedCodec(codec)) => {
        eprintln!("不支持的编解码器: {:?}", codec);
    }
    Err(e) => {
        eprintln!("其他错误: {:?}", e);
    }
}
```

---

## 参考

- [ITU-T G.711 标准](https://www.itu.int/rec/T-REC-G.711)
- [FFmpeg libavformat 文档](https://ffmpeg.org/doxygen/trunk/group__lavf.html)
- [FFmpeg AVSampleFormat](https://ffmpeg.org/doxygen/trunk/group__lavu__sampfmts.html)
- [RTP Payload Format for G.711 (RFC 3551)](https://tools.ietf.org/html/rfc3551)
