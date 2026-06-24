# OpenGL 渲染模块文档

## 概述

`render` 模块提供基于 OpenGL 3.3 的视频渲染能力，当前主要支持 **MJPEG (Motion JPEG)** 格式的视频流渲染。

### 模块架构

```
src-tauri/src/render/
├── mod.rs          # 模块入口，导出公共 API
├── error.rs        # 错误类型定义
├── shader.rs       # GLSL 着色器编译、链接、uniform 管理
├── texture.rs      # OpenGL 纹理创建、更新、销毁
├── mjpeg.rs        # MJPEG 流解析与 JPEG→RGBA 解码
└── renderer.rs     # GLFW 窗口 + 渲染循环（完整渲染器）
```

### 渲染管线

```
┌─────────────┐     ┌────────────┐     ┌──────────┐     ┌───────────┐
│ MJPEG 数据   │ ──▶ │ MjpegParser │ ──▶ │ JPEG 解码 │ ──▶ │ RGBA 数据  │
│ (文件/网络)  │     │ (帧提取)    │     │ (CPU)     │     │ (内存)     │
└─────────────┘     └────────────┘     └──────────┘     └─────┬─────┘
                                                               │
                                                               ▼
┌─────────────┐     ┌────────────┐     ┌──────────┐     ┌───────────┐
│ 显示器       │ ◀── │ OpenGL 渲染 │ ◀── │ 纹理上传  │ ◀── │Texture    │
│             │     │ (GPU)      │     │ (GPU)     │     │::update() │
└─────────────┘     └────────────┘     └──────────┘     └───────────┘
```

## 快速开始

### 1. 作为独立示例运行

```bash
# 查看示例代码
cat src-tauri/examples/render_mjpeg.rs

# 运行单帧测试模式（无需输入文件）
cd src-tauri
cargo run --example render_mjpeg

# 播放 MJPEG 文件（通常为 .avi 容器）
cargo run --example render_mjpeg -- /path/to/video.avi
```

### 2. 在代码中使用

```rust
use learn_tauri_lib::render::renderer::MjpegRenderer;
use std::sync::mpsc;

// 启动渲染器（自动在后台线程运行）
let (frame_tx, handle) = MjpegRenderer::spawn(1280, 720, "MJPEG 播放器")
    .expect("渲染器启动失败");

// 从网络或文件读取 MJPEG 帧，发送到渲染器
loop {
    let jpeg_frame: Vec<u8> = read_frame_from_source();
    if frame_tx.send(jpeg_frame).is_err() {
        break; // 渲染器已退出
    }
}

// 等待渲染线程结束
handle.join().unwrap().unwrap();
```

## 模块详解

### error.rs —— 错误类型

所有渲染相关错误统一使用 `RenderError` 枚举：

| 错误变体 | 触发场景 |
|---------|---------|
| `GlfwInit` | GLFW 库初始化失败 |
| `WindowCreate` | OpenGL 窗口创建失败 |
| `ShaderCompile` | 顶点/片段着色器 GLSL 编译失败 |
| `ProgramLink` | 着色器程序链接失败 |
| `MissingSoi` | MJPEG 流中找不到 SOI 标记 (0xFFD8) |
| `MissingEoi` | MJPEG 流中找不到 EOI 标记 (0xFFD9) |
| `JpegDecode` | JPEG 数据解码失败（数据损坏或不支持格式） |
| `TextureError` | 纹理创建/更新失败 |

### shader.rs —— 着色器管理

内置两套着色器：

1. **默认 RGBA 渲染** (`VERTEX_SHADER_SRC` + `FRAGMENT_SHADER_SRC`)
   - 从 RGBA 纹理直接采样输出
   - 适用于 `image` crate 解码后的数据

2. **YUV→RGB 转换** (`FRAGMENT_SHADER_YUV_SRC`)
   - 在 GPU 上完成 YUV420P 到 RGB 的色彩空间转换
   - 适用于原生 YUV 视频帧（如摄像头原始数据）

```rust
use learn_tauri_lib::render::shader::ShaderProgram;

// 创建一个着色器程序
let shader = ShaderProgram::new(VERTEX_SRC, FRAGMENT_SRC)?;

// 激活使用
shader.use_program();

// 设置 uniform 变量
shader.set_uniform_1i("ourTexture", 0);
shader.set_uniform_4f("ourColor", 1.0, 0.5, 0.2, 1.0);
```

### texture.rs —— 纹理管理

```rust
use learn_tauri_lib::render::texture::Texture;

// 从 RGBA 数据创建纹理
let tex = Texture::from_rgba(&rgba_data, width, height)?;

// 更新纹理数据（高效，适合视频帧刷新）
tex.update(&new_rgba_data, new_width, new_height);

// 绑定到纹理单元进行渲染
tex.bind(gl::TEXTURE0);
```

**性能说明：**
- `update()` 在尺寸不变时使用 `glTexSubImage2D`，只更新像素数据，不重新分配 GPU 内存
- 尺寸变化时会自动调用 `glTexImage2D` 重新分配

### mjpeg.rs —— MJPEG 流解析与解码

**MJPEG 格式：**
MJPEG 本质上是连续的 JPEG 图片流，每帧以 SOI (0xFFD8) 开始，以 EOI (0xFFD9) 结束。

```text
[0xFFD8][JPEG 图片数据][0xFFD9][0xFFD8][下一帧 JPEG 数据][0xFFD9]...
```

**使用方式 1：流式解析**

```rust
use learn_tauri_lib::render::mjpeg::MjpegParser;

let mut parser = MjpegParser::new();

// 持续喂入数据（适合网络流）
parser.feed(&network_data);

// 提取完整帧
while let Some(frame) = parser.next_frame() {
    let (rgba, w, h) = parser.decode_to_rgba(frame)?;
    // 处理 RGBA 数据...
}
```

**使用方式 2：独立解码**

```rust
use learn_tauri_lib::render::mjpeg::decode_jpeg_to_rgba;

let frame = decode_jpeg_to_rgba(&jpeg_file_data)?;
println!("解码成功: {}x{}", frame.width, frame.height);
```

### renderer.rs —— 完整渲染器

`MjpegRenderer` 在一个独立线程中运行完整的 OpenGL 渲染循环。

**架构：**

```
主线程                       渲染线程
───────                      ────────
frame_tx ──send(jpeg)──▶     receiver
                              │
                         [JPEG→RGBA]
                              │
                         [纹理更新]
                              │
                         [OpenGL 渲染]
```

**初始化参数：**

| 参数 | 说明 |
|------|------|
| `width` | 窗口初始宽度（像素） |
| `height` | 窗口初始高度（像素） |
| `title` | 窗口标题 |

**返回值：**

| 返回值 | 说明 |
|--------|------|
| `Sender<Vec<u8>>` | 发送 JPEG 帧数据的通道 |
| `JoinHandle` | 渲染线程句柄，用于等待退出 |

## 进阶用法

### 与 Tauri 集成

在 Tauri 命令中控制渲染器：

```rust
// src-tauri/src/lib.rs 或自定义命令

use std::sync::mpsc::Sender;
use std::sync::Mutex;
use learn_tauri_lib::render::renderer::MjpegRenderer;

// 全局状态
struct AppState {
    frame_tx: Mutex<Option<Sender<Vec<u8>>>>,
}

#[tauri::command]
async fn start_renderer(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let (tx, handle) = MjpegRenderer::spawn(1280, 720, "MJPEG")
        .map_err(|e| e.to_string())?;

    *state.frame_tx.lock().unwrap() = Some(tx);
    // handle 可存储用于后续停止
    Ok(())
}

#[tauri::command]
async fn send_frame(state: tauri::State<'_, AppState>, data: Vec<u8>) -> Result<(), String> {
    let tx = state.frame_tx.lock().unwrap();
    if let Some(ref tx) = *tx {
        tx.send(data).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

### YUV 渲染

如果视频源是 YUV420P 格式（常见于摄像头/硬件解码器），可以使用 YUV 着色器：

```rust
use learn_tauri_lib::render::shader::{ShaderProgram, FRAGMENT_SHADER_YUV_SRC, VERTEX_SHADER_SRC};

// 创建 YUV 着色器
let yuv_shader = ShaderProgram::new(VERTEX_SHADER_SRC, FRAGMENT_SHADER_YUV_SRC)?;

// 需要三个纹理分别对应 Y/U/V 分量
// 然后在渲染循环中绑定三个纹理到不同纹理单元
```

### 性能优化与卡顿解决

帧接收采用了 **"排空取新"** 策略来消除播放卡顿：

```rust
// 排空通道，但只保留最后收到的帧
let mut latest_jpeg: Option<Vec<u8>> = None;
while let Ok(jpeg_data) = self.receiver.try_recv() {
    latest_jpeg = Some(jpeg_data); // 旧帧直接丢弃
}

// 每帧循环只解码一次（最新的那帧）
if let Some(jpeg_data) = latest_jpeg {
    match decode_jpeg_to_rgba(&jpeg_data) { ... }
}
```

**原理：** 当 JPEG 解码速度跟不上视频帧率时，积压的帧会排队等待解码。等轮到它们时画面早已过时，白白浪费 CPU。该策略每轮循环只解码最新帧，旧帧直接丢弃，确保 CPU 专注于处理当前需要显示的画面。

| 场景 | 建议 |
|------|------|
| 高帧率 (>30fps) | 保持纹理尺寸不变，避免重新分配 GPU 内存 |
| 大分辨率 (4K) | 考虑使用 GPU 端 YUV→RGB 转换（YUV 着色器） |
| 低延迟 | 使用 `try_recv()` 非阻塞接收，避免渲染阻塞 |
| 多路视频 | 每个视频流创建独立的 `MjpegRenderer` 或共享一个渲染器轮播 |
| CPU 占用 | 无帧时自动 sleep(16ms)，降低空转功耗 |

## 常见问题

**Q: 窗口没有显示任何内容？**
- 确认已发送帧数据且解码成功
- 检查 JPEG 文件是否是有效的格式
- 查看控制台是否有解码错误输出

**Q: 渲染很卡/掉帧？**
- 检查 JPEG 解码性能（大分辨率图片解码耗时高）
- 减小图片分辨率或优化解码路径
- 使用 `try_recv()` 丢弃积压帧，只渲染最新的帧

**Q: 如何关闭渲染器？**
- 关闭窗口（点击 X 或按 ESC 键）
- 销毁 Sender（drop frame_tx），渲染器会在下一轮检查时退出

**Q: 支持哪些 OpenGL 版本？**
- 需要 OpenGL 3.3+（Core Profile）
- Windows/Linux/macOS 均可运行
