# 录像模块指导说明文档

## 目录

1. [概述](#1-概述)
2. [安装和配置](#2-安装和配置)
3. [快速开始](#3-快速开始)
4. [API 参考](#4-api-参考)
5. [示例代码](#5-示例代码)
6. [常见问题](#6-常见问题)
7. [C++/FFmpeg FFI 接口说明](#7-cffmpeg-ffi-接口说明)
8. [性能优化建议](#8-性能优化建议)
9. [故障排除](#9-故障排除)

---

## 1. 概述

### 1.1 功能介绍

录像模块提供了将 RTP/AVTP 流保存为视频文件的功能，支持以下特性：

- **支持的编解码器**：
  - H.264 (AVC)
  - H.265 (HEVC)
  - MJPEG (Motion JPEG)

- **支持的容器格式**：
  - MP4 (支持 H.264/H.265)
  - AVI (支持 MJPEG)

- **核心功能**：
  - 开始/结束录像
  - 写入视频帧（支持时间戳）
  - 取消录像（不保存文件）
  - 获取录像统计信息（帧数、字节数、持续时间等）
  - 记录开始/结束时间戳

### 1.2 架构设计

录像模块采用 trait 对象设计，提供统一的 `Recorder` trait 接口：

```
┌─────────────────────────────────────────────────────────┐
│                  应用程序代码                              │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                Recorder trait (统一接口)                 │
│  - start()                                              │
│  - write_frame()                                        │
│  - write_media_frame()                                  │
│  - finish()                                             │
│  - cancel()                                             │
│  - get_stats()                                          │
└─────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        ▼                                       ▼
┌──────────────────┐                    ┌──────────────────┐
│   Mp4Recorder     │                   │    AviRecorder   │
│  (MP4 封装)       │                    │  (AVI 封装)     │
└──────────────────┘                    └──────────────────┘
        │                                       │
        └───────────────────┬───────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────┐
│              FfiRecorder (FFI 包装器)                   │
│  - 对接 C++/FFmpeg 实现视频封装                          │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              C++ 端实现 (recorder_ffi.cpp)                │
│  - 使用 FFmpeg libavformat 库进行封装                     │
└─────────────────────────────────────────────────────────┘
```

### 1.3 模块结构

```
src-tauri/src/recorder/
├── mod.rs          # 模块入口，公开导出
├── error.rs        # 错误类型定义
├── config.rs       # 配置结构定义
├── trait_.rs       # Recorder trait 定义
├── ffi.rs         # FFI 接口定义
├── mp4.rs         # MP4 录像器实现
└── avi.rs         # AVI 录像器实现
```

---

## 2. 安装和配置

### 2.1 启用 Feature

录像模块使用 Cargo feature 进行控制，需要在 `Cargo.toml` 中启用对应的 feature：

```toml
[dependencies]
learn_tauri = { path = "../path/to/learn_tauri", features = ["recorder-mp4", "recorder-avi"] }
```

**可用的 feature**：

| Feature | 说明 | 依赖 |
|---------|------|------|
| `recorder-ffi` | 启用 FFI 录像器实现（对接 C++/FFmpeg） | 需要 C++ 端实现 |
| `recorder-mp4` | 启用 MP4 录像器（支持 H.264/H.265） | `recorder-ffi` |
| `recorder-avi` | 启用 AVI 录像器（支持 MJPEG） | `recorder-ffi` |

### 2.2 安装 FFmpeg (C++ 端)

录像模块使用 FFI 对接 C++/FFmpeg 进行视频封装，需要：

1. **安装 FFmpeg 库**：
   - Windows: 下载 FFmpeg 开发库（包含头文件和库文件）
   - Linux: `sudo apt install libavformat-dev libavcodec-dev libavutil-dev`
   - macOS: `brew install ffmpeg`

2. **编译 C++ 端实现**：
   - 实现 `recorder_ffi.cpp`（参考第 7 节）
   - 编译为静态库或动态库
   - 在 `build.rs` 中链接该库

### 2.3 配置 build.rs

在 `build.rs` 中添加 FFmpeg 库的链接：

```rust
// build.rs
fn main() {
    // 链接 FFmpeg 库
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avutil");

    // 如果需要链接 C++ 端实现
    println!("cargo:rustc-link-lib=recorder_ffi");
    println!("cargo:rustc-link-search=native=./cpp_lib"); // 库文件路径
}
```

---

## 3. 快速开始

### 3.1 基本用法（MP4 录像）

```rust
use recorder::{Mp4Recorder, Recorder};
use recorder::config::RecorderConfig;
use rtp::decoder::CodecType;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建录像器
    let mut recorder = Mp4Recorder::new(
        CodecType::H264,
        PathBuf::from("output.mp4"),
    )?;

    // 2. 开始录像
    let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"))
        .with_width(1920)
        .with_height(1080)
        .with_framerate(30.0);

    recorder.start(config)?;

    // 3. 写入视频帧
    let frame_data = vec![0u8; 1024]; // 实际的视频帧数据
    recorder.write_frame(&frame_data, Some(100))?; // 时间戳 100ms

    // 4. 结束录像
    recorder.finish()?;

    // 5. 获取统计信息
    let stats = recorder.get_stats();
    println!("录像完成: {} 帧, {} 字节", stats.frames_written, stats.bytes_written);

    Ok(())
}
```

### 3.2 基本用法（AVI 录像）

```rust
use recorder::{AviRecorder, Recorder};
use recorder::config::RecorderConfig;
use rtp::decoder::CodecType;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建录像器
    let mut recorder = AviRecorder::new(
        CodecType::MJPEG,
        PathBuf::from("output.avi"),
    )?;

    // 2. 开始录像
    let config = RecorderConfig::new(CodecType::MJPEG, PathBuf::from("output.avi"))
        .with_width(1920)
        .with_height(1080)
        .with_framerate(30.0);

    recorder.start(config)?;

    // 3. 写入视频帧
    let jpeg_data = vec![0u8; 1024]; // 实际的 JPEG 数据
    recorder.write_frame(&jpeg_data, Some(100))?;

    // 4. 结束录像
    recorder.finish()?;

    Ok(())
}
```

### 3.3 使用 MediaFrame

如果已经有 `MediaFrame` 对象（从 RTP 解析器获得），可以直接使用 `write_media_frame()`：

```rust
use recorder::Recorder;
use rtp::decoder::frame::MediaFrame;

// 从 RTP 解析器获得 MediaFrame
let media_frame = MediaFrame {
    data: vec![0u8; 1024],
    pts: 100000, // 微秒
    dts: 100000,
    keyframe: true,
    codec_type: CodecType::H264,
    width: 1920,
    height: 1080,
    ..Default::default()
};

// 写入 MediaFrame（自动转换时间戳）
recorder.write_media_frame(&media_frame)?;
```

---

## 4. API 参考

### 4.1 Recorder trait

```rust
pub trait Recorder {
    /// 开始录像
    fn start(&mut self, config: RecorderConfig) -> RecordResult<()>;

    /// 写入视频帧
    fn write_frame(&mut self, frame: &[u8], timestamp_ms: Option<u64>) -> RecordResult<()>;

    /// 写入 MediaFrame（自动转换时间戳）
    fn write_media_frame(&mut self, frame: &MediaFrame) -> RecordResult<()>;

    /// 结束录像
    fn finish(&mut self) -> RecordResult<()>;

    /// 获取录像统计信息
    fn get_stats(&self) -> RecordStats;

    /// 检查是否正在录像
    fn is_recording(&self) -> bool;

    /// 获取当前配置
    fn get_config(&self) -> Option<&RecorderConfig>;

    /// 取消录像（不保存文件）
    fn cancel(&mut self) -> RecordResult<()>;
}
```

### 4.2 RecorderConfig

```rust
pub struct RecorderConfig {
    pub codec_type: CodecType,       // 编解码器类型
    pub output_path: PathBuf,         // 输出文件路径
    pub width: u32,                  // 视频宽度
    pub height: u32,                 // 视频高度
    pub framerate: f64,              // 帧率
}

impl RecorderConfig {
    pub fn new(codec_type: CodecType, output_path: PathBuf) -> Self;
    pub fn with_container_format(self, format: ContainerFormat) -> Self;
    pub fn with_width(self, width: u32) -> Self;
    pub fn with_height(self, height: u32) -> Self;
    pub fn with_framerate(self, framerate: f64) -> Self;
    pub fn get_container_format(&self) -> Option<ContainerFormat>;
    pub fn validate(&self) -> Result<(), String>;
}
```

### 4.3 RecordStats

```rust
pub struct RecordStats {
    pub start_time: Option<SystemTime>,       // 开始时间（系统时间）
    pub end_time: Option<SystemTime>,         // 结束时间（系统时间）
    pub start_timestamp_ms: Option<u64>,      // 开始时间戳（Unix 毫秒）
    pub end_timestamp_ms: Option<u64>,        // 结束时间戳（Unix 毫秒）
    pub frames_written: u64,                 // 写入的帧数
    pub bytes_written: u64,                  // 写入的字节数
    pub duration_ms: Option<u64>,            // 录像持续时间（毫秒）
}
```

### 4.4 RecordError

```rust
pub enum RecordError {
    InitError(String),      // 初始化失败
    WriteError(String),      // 写入失败
    InvalidArgument(String), // 无效参数
    Unsupported(String),     // 不支持的操作
}
```

### 4.5 辅助函数

```rust
// 创建录像器（自动选择合适的实现）
pub fn create_recorder(config: &RecorderConfig) -> RecordResult<Box<dyn Recorder + Send>>;

// 检查编解码器是否支持录像
pub fn is_codec_supported(codec: CodecType) -> bool;

// 获取支持的编解码器列表
pub fn supported_codecs() -> Vec<CodecType>;

// 获取支持的容器格式列表
pub fn supported_containers() -> Vec<ContainerFormat>;
```

---

## 5. 示例代码

### 5.1 从 RTP 流录像

```rust
use recorder::{Mp4Recorder, Recorder};
use recorder::config::RecorderConfig;
use rtp::decoder::{CodecType, Parser};
use std::path::PathBuf;

fn record_from_rtp_stream(rtp_packets: Vec<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建录像器
    let mut recorder = Mp4Recorder::new(
        CodecType::H264,
        PathBuf::from("output.mp4"),
    )?;

    // 2. 开始录像
    let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"))
        .with_width(1920)
        .with_height(1080)
        .with_framerate(30.0);

    recorder.start(config)?;

    // 3. 解析 RTP 数据包并写入
    let mut parser = Parser::new(CodecType::H264);
    for (i, rtp_packet) in rtp_packets.iter().enumerate() {
        // 解析 RTP 数据包
        if let Some(media_frame) = parser.parse(rtp_packet)? {
            // 写入视频帧
            recorder.write_media_frame(&media_frame)?;

            if (i + 1) % 100 == 0 {
                println!("已写入 {} 帧", i + 1);
            }
        }
    }

    // 4. 结束录像
    recorder.finish()?;

    // 5. 打印统计信息
    let stats = recorder.get_stats();
    println!("录像完成:");
    println!("  开始时间: {:?}", stats.start_time);
    println!("  结束时间: {:?}", stats.end_time);
    println!("  持续时间: {:?} ms", stats.duration_ms);
    println!("  写入帧数: {}", stats.frames_written);
    println!("  写入字节数: {}", stats.bytes_written);

    Ok(())
}
```

### 5.2 定时录像（带时间戳）

```rust
use recorder::{Mp4Recorder, Recorder};
use recorder::config::RecorderConfig;
use rtp::decoder::CodecType;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn record_with_timestamp() -> Result<(), Box<dyn std::error::Error>> {
    let mut recorder = Mp4Recorder::new(
        CodecType::H264,
        PathBuf::from("output.mp4"),
    )?;

    let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"))
        .with_framerate(30.0);

    recorder.start(config)?;

    // 模拟写入 300 帧（10 秒）
    for i in 0..300 {
        let frame_data = generate_frame_data(i); // 生成视频帧数据

        // 计算时间戳（毫秒）
        let timestamp_ms = (i as f64 * 1000.0 / 30.0) as u64;

        recorder.write_frame(&frame_data, Some(timestamp_ms))?;

        if (i + 1) % 30 == 0 {
            println!("已录像 {} 秒 ({} 帧)", (i + 1) / 30, i + 1);
        }
    }

    recorder.finish()?;

    Ok(())
}
```

### 5.3 取消录像

```rust
use recorder::{Mp4Recorder, Recorder};
use recorder::config::RecorderConfig;
use rtp::decoder::CodecType;
use std::path::PathBuf;

fn record_with_cancel() -> Result<(), Box<dyn std::error::Error>> {
    let mut recorder = Mp4Recorder::new(
        CodecType::H264,
        PathBuf::from("output.mp4"),
    )?;

    let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"));
    recorder.start(config)?;

    // 写入一些帧
    for i in 0..100 {
        let frame_data = vec![0u8; 1024];
        recorder.write_frame(&frame_data, Some(i * 33))?;
    }

    // 决定取消录像（不保存文件）
    if some_condition() {
        recorder.cancel()?;
        println!("录像已取消，文件未保存");
        return Ok(());
    }

    // 否则继续写入并完成
    for i in 100..300 {
        let frame_data = vec![0u8; 1024];
        recorder.write_frame(&frame_data, Some(i * 33))?;
    }

    recorder.finish()?;
    println!("录像已完成");

    Ok(())
}
```

---

## 6. 常见问题

### 6.1 编译错误：无法找到 `recorder` 模块

**问题**：编译时出现 `error[E0432]: unresolved import `recorder``

**解决方案**：
1. 确保在 `Cargo.toml` 中启用了对应的 feature：
   ```toml
   [dependencies]
   learn_tauri = { features = ["recorder-mp4"] }
   ```

2. 确保在 `lib.rs` 中声明了 `recorder` 模块：
   ```rust
   pub mod recorder;
   ```

### 6.2 运行时错误：FFI 调用失败

**问题**：调用 `Mp4Recorder::new()` 或 `write_frame()` 时出现 `InitError` 或 `WriteError`

**可能原因**：
1. C++ 端实现未正确编译或链接
2. FFmpeg 库未安装或路径不正确
3. 输出文件路径无效（目录不存在）

**解决方案**：
1. 检查 C++ 端实现是否正确编译
2. 检查 `build.rs` 中的库链接配置
3. 确保输出文件的目录存在

### 6.3 录像文件无法播放

**问题**：生成的 MP4/AVI 文件无法用播放器打开

**可能原因**：
1. 视频帧数据格式不正确
2. 关键帧（IDR 帧）缺失
3. 时间戳不连续

**解决方案**：
1. 确保写入的视频帧数据是完整的（包含 NALU 起始码）
2. 确保定期写入关键帧
3. 使用连续的时间戳

### 6.4 性能问题

**问题**：录像时 CPU 使用率过高或帧率不稳定

**解决方案**：
1. 使用 Release 模式编译（`cargo build --release`）
2. 减少不必要的内存拷贝
3. 使用异步写入（如果支持）

---

## 7. C++/FFmpeg FFI 接口说明

### 7.1 接口函数

C++ 端需要实现以下 C 风格接口：

```cpp
// 创建录像器
RecorderHandle recorder_create(int codec_type, int container_format, const char* output_path);

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

// 获取录像统计信息
int recorder_get_stats(RecorderHandle handle, RecorderStatsFFI* stats);
```

### 7.2 参数说明

| 参数 | 类型 | 说明 |
|------|------|------|
| `codec_type` | `int` | 编解码器类型（0=H264, 1=H265, 2=MJPEG） |
| `container_format` | `int` | 容器格式（0=MP4, 1=AVI） |
| `output_path` | `const char*` | 输出文件路径（UTF-8 字符串） |
| `width` | `int` | 视频宽度（0 表示未知） |
| `height` | `int` | 视频高度（0 表示未知） |
| `framerate` | `double` | 帧率（0.0 表示未知） |
| `data` | `const uint8_t*` | 视频帧数据 |
| `size` | `size_t` | 数据大小（字节） |
| `timestamp_ms` | `int64_t` | 时间戳（毫秒，0 表示未知） |
| `keyframe` | `int` | 是否为关键帧（1=是，0=否） |

### 7.3 返回值

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `recorder_create()` | `RecorderHandle` | 成功：非空指针；失败：NULL |
| 其他函数 | `int` | 成功：0；失败：非 0 错误码 |

### 7.4 示例代码（C++ 端）

```cpp
#include <cstring>
#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/avutil.h>

struct RecorderContext {
    AVFormatContext* fmt_ctx;
    AVStream* video_stream;
    int64_t start_time;
    int64_t frame_count;
};

extern "C" {
    RecorderHandle recorder_create(int codec_type, int container_format, const char* output_path) {
        auto* ctx = new RecorderContext();
        // 初始化 FFmpeg，创建 MP4/AVI 文件
        // ...
        return static_cast<RecorderHandle>(ctx);
    }

    void recorder_destroy(RecorderHandle handle) {
        auto* ctx = static_cast<RecorderContext*>(handle);
        // 释放资源
        // ...
        delete ctx;
    }

    int recorder_start(RecorderHandle handle, int width, int height, double framerate) {
        auto* ctx = static_cast<RecorderContext*>(handle);
        // 写入文件头
        // ...
        return 0;
    }

    int recorder_write_frame(RecorderHandle handle, const uint8_t* data, size_t size,
                           int64_t timestamp_ms, int keyframe) {
        auto* ctx = static_cast<RecorderContext*>(handle);
        // 写入视频帧
        // ...
        return 0;
    }

    int recorder_finish(RecorderHandle handle) {
        auto* ctx = static_cast<RecorderContext*>(handle);
        // 写入文件尾
        // ...
        return 0;
    }

    int recorder_cancel(RecorderHandle handle) {
        // 取消录像（不保存文件）
        // ...
        return 0;
    }

    int recorder_get_stats(RecorderHandle handle, RecorderStatsFFI* stats) {
        auto* ctx = static_cast<RecorderContext*>(handle);
        // 填充统计信息
        // ...
        return 0;
    }
}
```

---

## 8. 性能优化建议

### 8.1 使用 Release 模式

```bash
cargo build --release
```

Release 模式会启用优化，显著提升性能。

### 8.2 减少内存拷贝

- 使用引用而不是拷贝：`write_frame(&frame_data, ...)`
- 避免不必要的 `vec![]` 分配

### 8.3 批量写入

如果可能，批量写入多个视频帧（需要 C++ 端支持）。

### 8.4 使用异步 I/O

如果录像和 RTP 接收在同一个线程，考虑使用异步 I/O 避免阻塞。

---

## 9. 故障排除

### 9.1 启用日志

在 `Cargo.toml` 中启用日志功能，然后在代码中添加日志：

```rust
use log::{info, warn, error};

info!("开始录像: {:?}", config);
warn!("时间戳不连续: {} -> {}", last_timestamp, current_timestamp);
error!("写入帧失败: {}", e);
```

### 9.2 检查 FFmpeg 版本

确保安装了正确版本的 FFmpeg 库：

```bash
ffmpeg -version
```

### 9.3 使用示例程序测试

运行示例程序进行调试：

```bash
cargo run --bin recorder_example --features recorder-mp4 -- --codec h264 --output test.mp4
```

### 9.4 联系支持

如果遇到无法解决的问题，请提供以下信息：
- 错误信息
- 代码片段
- FFmpeg 版本
- 操作系统信息

---

## 附录 A：完整示例代码

参见 `src/bin/recorder_example.rs`

## 附录 B：配置文件示例

```toml
# Cargo.toml
[dependencies]
learn_tauri = { path = "../learn_tauri", features = ["recorder-mp4", "recorder-avi"] }

# build.rs
fn main() {
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avutil");
}
```

---

**文档版本**：1.0  
**最后更新**：2026-06-25  
**作者**：Learn Tauri Project
