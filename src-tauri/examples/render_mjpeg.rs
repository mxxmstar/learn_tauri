//! MJPEG 视频渲染示例
//!
//! 本示例演示如何使用 render 模块完整播放 MJPEG 视频流：
//! 1. 从文件或网络读取 MJPEG 数据
//! 2. 使用 MjpegParser 解析帧
//! 3. 使用 MjpegRenderer 渲染到 OpenGL 窗口
//!
//! 运行方式：
//! ```bash
//! cd src-tauri
//! cargo run --example render_mjpeg -- <路径/到/测试.avi>
//! ```
//!
//! 如果没有指定文件，会生成一张测试 JPEG 图片模拟单帧渲染。

use std::env;
use std::fs;

use std::thread;
use std::time::Duration;

/// 引入渲染模块（通过 lib crate 路径）
use learn_tauri_lib::render::mjpeg::{MjpegParser, decode_jpeg_to_rgba};
use learn_tauri_lib::render::renderer::MjpegRenderer;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        // ===== 模式 1: 播放 MJPEG 文件 =====
        let file_path = &args[1];
        play_mjpeg_file(file_path);
    } else {
        // ===== 模式 2: 单帧测试 =====
        single_frame_test();
    }
}

/// 播放 MJPEG 文件
///
/// 读取 MJPEG 文件，在新线程中启动渲染器，然后通过通道发送帧数据。
fn play_mjpeg_file(file_path: &str) {
    println!("📖 读取 MJPEG 文件: {}", file_path);

    // 读取文件内容
    let file_data = fs::read(file_path).expect("文件读取失败");
    println!("📦 文件大小: {} 字节", file_data.len());

    // 启动渲染器（在新线程中）
    let (frame_tx, render_handle) = MjpegRenderer::spawn(1280, 720, "MJPEG 播放器")
        .expect("渲染器启动失败");

    // 解析 MJPEG 流并逐帧发送
    let mut parser = MjpegParser::new();
    parser.feed(&file_data);

    println!("🎬 开始播放...");
    let mut frame_count = 0;

    // 提取帧并发送到渲染器
    while let Some(jpeg_frame) = parser.next_frame() {
        // 将帧数据转为 Vec<u8> 以便跨线程发送
        let frame_data = jpeg_frame.to_vec();
        if frame_tx.send(frame_data).is_err() {
            eprintln!("渲染器已关闭，停止发送");
            break;
        }
        frame_count += 1;

        // 模拟 30fps 的帧间隔
        thread::sleep(Duration::from_millis(33));
    }

    println!("✅ 共发送 {} 帧", frame_count);
    println!("⌛ 等待渲染器退出...");

    // 等待渲染器窗口关闭
    match render_handle.join() {
        Ok(Ok(_)) => println!("渲染器正常退出"),
        Ok(Err(e)) => eprintln!("渲染器异常退出: {}", e),
        Err(_) => eprintln!("渲染线程 panic"),
    }
}

/// 单帧测试
///
/// 创建一个窗口并用测试图案填充，验证渲染管线是否正常工作。
/// 因为没有实际的 MJPEG 数据，我们使用本地生成的一张测试 JPEG 图片。
fn single_frame_test() {
    println!("🖼️  单帧测试模式（无输入文件）");

    // 生成测试图片
    let test_image = create_test_jpeg(320, 240);
    println!("📝 生成测试图片: {} 字节", test_image.len());

    // 解码验证
    match decode_jpeg_to_rgba(&test_image) {
        Ok(frame) => {
            println!(
                "✅ 解码成功: {}x{}, RGBA 数据 {} 字节",
                frame.width,
                frame.height,
                frame.rgba.len()
            );
        }
        Err(e) => {
            eprintln!("❌ 解码失败: {} (使用内嵌测试图案)", e);
            println!("💡 可以指定 MJPEG 文件路径作为参数运行此示例");
            println!("   例如: cargo run --example render_mjpeg -- test.avi");
            return;
        }
    }

    // 创建渲染器并显示测试图
    let (frame_tx, render_handle) = MjpegRenderer::spawn(800, 600, "MJPEG 单帧测试")
        .expect("渲染器启动失败");

    // 发送测试帧
    frame_tx.send(test_image).ok();

    println!("🔄 渲染窗口已打开，关闭窗口后退出...");

    match render_handle.join() {
        Ok(Ok(_)) => println!("渲染器正常退出"),
        Ok(Err(e)) => eprintln!("渲染器异常退出: {}", e),
        Err(_) => eprintln!("渲染线程 panic"),
    }
}

/// 创建一个测试用的 JPEG 图片
///
/// 生成一张带有颜色渐变和文字标记的 JPEG 图片，
/// 用于在没有外部文件的情况下验证渲染管线。
fn create_test_jpeg(width: u32, height: u32) -> Vec<u8> {
    // 生成 RGB 渐变图案
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = 128u8;
            rgb.push(r);
            rgb.push(g);
            rgb.push(b);
        }
    }

    // 使用 image crate 编码为 JPEG
    let mut jpeg_data = Vec::new();
    {
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut jpeg_data);
        encoder
            .encode(&rgb, width, height, image::ColorType::Rgb8.into())
            .expect("JPEG 编码失败");
    }

    jpeg_data
}
