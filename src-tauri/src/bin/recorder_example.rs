//! 录像模块示例程序
//!
//! 演示如何使用录像模块将 RTP/AVTP 流保存为 MP4 或 AVI 文件
//!
//! # 使用方法
//!
//! ```bash
//! # 运行 MP4 录像示例（H.264）
//! cargo run --bin recorder_example --features recorder-mp4 -- --codec h264 --output output.mp4
//!
//! # 运行 AVI 录像示例（MJPEG）
//! cargo run --bin recorder_example --features recorder-avi -- --codec mjpeg --output output.avi
//! ```

use std::env;
use std::path::PathBuf;

use learn_tauri_lib::recorder::{create_recorder, Recorder, RecorderConfig};
use learn_tauri_lib::rtp::decoder::CodecType;

/// 应用程序配置
struct App {
    codec_type: CodecType,
    output_path: PathBuf,
    width: u32,
    height: u32,
    framerate: f64,
}

impl App {
    /// 从命令行参数解析配置
    fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut codec_str = "h264".to_string();
        let mut output_path = "output.mp4".to_string();
        let mut width: u32 = 1920;
        let mut height: u32 = 1080;
        let mut framerate: f64 = 30.0;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--codec" | "-c" => {
                    if i + 1 < args.len() {
                        codec_str = args[i + 1].clone();
                        i += 2;
                    } else { i += 1; }
                }
                "--output" | "-o" => {
                    if i + 1 < args.len() {
                        output_path = args[i + 1].clone();
                        i += 2;
                    } else { i += 1; }
                }
                "--width" | "-W" => {
                    if i + 1 < args.len() {
                        width = args[i + 1].parse().unwrap_or(1920);
                        i += 2;
                    } else { i += 1; }
                }
                "--height" | "-H" => {
                    if i + 1 < args.len() {
                        height = args[i + 1].parse().unwrap_or(1080);
                        i += 2;
                    } else { i += 1; }
                }
                "--framerate" | "-r" => {
                    if i + 1 < args.len() {
                        framerate = args[i + 1].parse().unwrap_or(30.0);
                        i += 2;
                    } else { i += 1; }
                }
                "--help" | "-h" => {
                    print_usage(&args[0]);
                    std::process::exit(0);
                }
                _ => { i += 1; }
            }
        }

        let codec_type = match codec_str.as_str() {
            "h264" => CodecType::H264,
            "h265" => CodecType::H265,
            "mjpeg" => CodecType::MJPEG,
            other => {
                eprintln!("未知的编解码器: {}", other);
                std::process::exit(1);
            }
        };

        Self {
            codec_type,
            output_path: PathBuf::from(output_path),
            width,
            height,
            framerate,
        }
    }

    /// 运行录像示例
    fn run(&self) -> Result<(), String> {
        println!("录像模块示例");
        println!("  编解码器: {:?}", self.codec_type);
        println!("  输出文件: {}", self.output_path.display());
        println!("  视频尺寸: {}x{}", self.width, self.height);
        println!("  帧率: {} fps", self.framerate);

        // 1. 创建录像配置
        let config = RecorderConfig::new(self.codec_type, self.output_path.clone())
            .with_dimensions(self.width, self.height)
            .with_framerate(self.framerate);

        // 打印容器格式
        if let Some(container) = config.get_container_format() {
            println!("  容器格式: {}", container);
        }

        // 2. 创建录像器
        let mut recorder = create_recorder(&config)
            .map_err(|e| format!("创建录像器失败: {}", e))?;

        println!("\n录像器创建成功");

        // 3. 开始录像
        recorder.start(config.clone())
            .map_err(|e| format!("开始录像失败: {}", e))?;

        println!("开始录像...");

        // 4. 写入模拟视频帧
        self.write_dummy_frames(&mut *recorder)?;

        // 5. 结束录像
        recorder.finish()
            .map_err(|e| format!("结束录像失败: {}", e))?;

        println!("\n录像完成");

        // 6. 打印统计信息
        let stats = recorder.get_stats();
        println!("\n录像统计信息:");
        println!("  开始时间: {:?}", stats.start_time);
        println!("  结束时间: {:?}", stats.end_time);
        println!("  持续时间: {:?} ms", stats.duration_ms);
        println!("  写入帧数: {}", stats.frames_written);
        println!("  写入字节数: {}", stats.bytes_written);

        Ok(())
    }

    /// 写入模拟视频帧
    fn write_dummy_frames(&self, recorder: &mut dyn Recorder) -> Result<(), String> {
        println!("写入模拟视频帧...");

        let frame_count = 100;
        let frame_size = 1024;

        for i in 0..frame_count {
            let frame_data = vec![0u8; frame_size];
            let timestamp_ms = if self.framerate > 0.0 {
                Some((i as f64 * 1000.0 / self.framerate) as u64)
            } else {
                None
            };

            recorder.write_frame(&frame_data, timestamp_ms)
                .map_err(|e| format!("写入帧 {} 失败: {}", i, e))?;

            if (i + 1) % 10 == 0 {
                println!("  已写入 {} 帧", i + 1);
            }
        }

        println!("写入完成，共 {} 帧", frame_count);
        Ok(())
    }
}

fn print_usage(prog: &str) {
    println!("录像模块示例程序");
    println!("\n用法: {} [选项]\n", prog);
    println!("选项:");
    println!("  -c, --codec <CODEC>     编解码器类型 (h264, h265, mjpeg) [默认: h264]");
    println!("  -o, --output <FILE>     输出文件路径 [默认: output.mp4]");
    println!("  -W, --width <WIDTH>     视频宽度 [默认: 1920]");
    println!("  -H, --height <HEIGHT>   视频高度 [默认: 1080]");
    println!("  -r, --framerate <FPS>   帧率 [默认: 30.0]");
    println!("  -h, --help              显示帮助信息");
    println!("\n示例:");
    println!("  {} --codec h264 --output output.mp4", prog);
    println!("  {} --codec mjpeg --output output.avi", prog);
}

fn main() {
    let app = App::parse();

    if let Err(e) = app.run() {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
