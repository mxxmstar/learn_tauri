//! MJPEG OpenGL 渲染器
//!
//! 提供完整的 MJPEG 视频渲染功能，包含：
//! - GLFW 窗口管理
//! - OpenGL 上下文初始化
//! - 着色器程序管理
//! - 纹理更新与渲染
//! - 多线程帧接收通道
//!
//! # 渲染管线
//!
//! ```text
//! [MJPEG 数据] ──→ [MjpegParser] ──→ [JPEG 解码] ──→ [RGBA 数据]
//!                                                         │
//!                                                         ▼
//! [显示器] ←── [OpenGL 渲染] ←── [纹理上传] ←── [Texture::update]
//! ```
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use learn_tauri_lib::render::renderer::MjpegRenderer;
//! use std::sync::mpsc;
//!
//! // 创建渲染器（在新线程中运行）
//! let (frame_tx, renderer) = MjpegRenderer::spawn(800, 600, "MJPEG 播放器")
//!     .expect("渲染器创建失败");
//!
//! // 在主线程中发送帧数据
//! let jpeg_data: Vec<u8> = vec![]; // 从网络或文件读取
//! frame_tx.send(jpeg_data).ok();
//!
//! // 等待渲染器退出
//! renderer.join().unwrap();
//! ```

use gl::types::*;
use glfw::{Action, Context, Key};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::error::RenderError;
use super::mjpeg::decode_jpeg_to_rgba;
use super::shader::{ShaderProgram, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC};
use super::texture::Texture;

/// 全屏四边形的顶点数据
///
/// 每个顶点包含 5 个分量：位置 (x, y, z) + 纹理坐标 (u, v)
/// 纹理坐标原点在左下角 (0,0)，右上角为 (1,1)
///
/// 四边形由两个三角形组成：
/// ```text
/// 0─────2
/// │ ╲   │
/// │   ╲ │
/// 1─────3
/// ```
const QUAD_VERTICES: [f32; 30] = [
    // 位置             纹理坐标
    // 第一个三角形
    -1.0,  1.0, 0.0,   0.0, 1.0, // 0: 左上
    -1.0, -1.0, 0.0,   0.0, 0.0, // 1: 左下
     1.0,  1.0, 0.0,   1.0, 1.0, // 2: 右上
    // 第二个三角形
    -1.0, -1.0, 0.0,   0.0, 0.0, // 1: 左下（重复）
     1.0, -1.0, 0.0,   1.0, 0.0, // 3: 右下
     1.0,  1.0, 0.0,   1.0, 1.0, // 2: 右上（重复）
];

/// MJPEG 渲染器
///
/// 负责管理 OpenGL 窗口、着色器、纹理和渲染循环。
/// 通过通道接收帧数据，在渲染线程中完成解码→纹理更新→渲染的全流程。
///
/// ## 架构设计
///
/// 渲染器在独立的线程中运行，与主线程通过 MPSC 通道通信：
///
/// ```text
/// 主线程                        渲染线程
/// ────────                      ────────
/// frame_tx.send(data) ──────→  receiver.recv()
///                                    │
///                               [JPEG→RGBA 解码]
///                                    │
///                               [纹理更新]
///                                    │
///                               [OpenGL 渲染]
/// ```
pub struct MjpegRenderer {
    /// GLFW 窗口，持有 OpenGL 上下文
    window: glfw::PWindow,
    /// GLFW 实例
    glfw: glfw::Glfw,
    /// 着色器程序
    shader: ShaderProgram,
    /// VAO（顶点数组对象）
    vao: GLuint,
    /// VBO（顶点缓冲对象）
    vbo: GLuint,
    /// 当前纹理（Option 表示可能尚未创建）
    texture: Option<Texture>,
    /// 帧接收通道
    receiver: Receiver<Vec<u8>>,
}

impl MjpegRenderer {
    /// 创建渲染器并在新线程中启动渲染循环
    ///
    /// # 参数
    /// - `width`:  窗口宽度（像素）
    /// - `height`: 窗口高度（像素）
    /// - `title`:  窗口标题
    ///
    /// # 返回值
    /// - `Ok((Sender<Vec<u8>>, JoinHandle<Result<(), RenderError>>))`:
    ///   发送帧数据的通道发送端 + 渲染线程句柄
    /// - `Err(RenderError)`: 初始化失败
    ///
    /// # 使用说明
    ///
    /// 调用方通过返回的 Sender 发送原始 JPEG 数据即可。
    /// 渲染器会自动完成解码、纹理更新和屏幕绘制。
    pub fn spawn(
        width: u32,
        height: u32,
        title: &str,
    ) -> Result<(Sender<Vec<u8>>, JoinHandle<Result<(), RenderError>>), RenderError> {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let title = title.to_string();

        let handle = thread::Builder::new()
            .name("mjpeg-renderer".into())
            .spawn(move || {
                let mut renderer = MjpegRenderer::new(width, height, &title, rx)?;
                renderer.run()
            })
            .map_err(|_| RenderError::ThreadJoin)?;

        Ok((tx, handle))
    }

    /// 创建新的渲染器实例
    ///
    /// 完成以下初始化：
    /// 1. 初始化 GLFW 并创建窗口
    /// 2. 加载 OpenGL 函数指针
    /// 3. 编译着色器程序
    /// 4. 创建顶点缓冲区
    fn new(width: u32, height: u32, title: &str, receiver: Receiver<Vec<u8>>) -> Result<Self, RenderError> {
        // ---- 1. 初始化 GLFW ----
        let mut glfw = glfw::init(glfw::fail_on_errors)
            .map_err(|e: glfw::InitError| RenderError::GlfwInit(e.to_string()))?;

        glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        #[cfg(target_os = "macos")]
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

        // ---- 2. 创建窗口 ----
        let (mut window, events) = glfw
            .create_window(
                width,
                height,
                title,
                glfw::WindowMode::Windowed,
            )
            .ok_or_else(|| RenderError::WindowCreate(format!("无法创建 {}x{} 窗口", width, height)))?;

        window.make_current();
        window.set_all_polling(true);
        glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

        // ---- 3. 加载 OpenGL 函数指针 ----
        gl::load_with(|symbol| {
            let addr = window.get_proc_address(symbol);
            match addr {
                Some(f) => f as *const std::ffi::c_void,
                None => std::ptr::null(),
            }
        });

        // ---- 4. 设置视口 ----
        unsafe {
            gl::Viewport(0, 0, width as GLsizei, height as GLsizei);
        }

        // ---- 5. 编译着色器 ----
        let shader = ShaderProgram::new(VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC)?;

        // ---- 6. 创建 VAO / VBO ----
        let (vao, vbo) = Self::create_quad_buffers()?;

        // ---- 启动事件处理线程（可选） ----
        // 事件接收器在单独的循环中处理，避免阻塞主渲染
        let _events = events; // 保持 receiver 活跃

        Ok(MjpegRenderer {
            window,
            glfw,
            shader,
            vao,
            vbo,
            texture: None,
            receiver,
        })
    }

    /// 创建全屏四边形的 VAO 和 VBO
    ///
    /// 顶点属性布局：
    /// - location = 0: 位置 (vec3) —— 偏移 0
    /// - location = 1: 纹理坐标 (vec2) —— 偏移 3 * sizeof(f32)
    fn create_quad_buffers() -> Result<(GLuint, GLuint), RenderError> {
        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (QUAD_VERTICES.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                QUAD_VERTICES.as_ptr() as *const GLvoid,
                gl::STATIC_DRAW,
            );

            // 顶点位置属性 (location = 0)
            let stride = 5 * std::mem::size_of::<f32>() as GLsizei;
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            // 纹理坐标属性 (location = 1)
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * std::mem::size_of::<f32>()) as *const GLvoid,
            );
            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        Ok((vao, vbo))
    }

    /// 主渲染循环
    ///
    /// 持续执行以下步骤直到窗口关闭：
    /// 1. 处理窗口事件
    /// 2. 从通道接收新的帧数据（非阻塞）
    /// 3. 如果收到新帧，解码并更新纹理
    /// 4. 清屏并绘制纹理
    /// 5. 交换缓冲区
    fn run(&mut self) -> Result<(), RenderError> {
        let mut has_frame = false;

        while !self.window.should_close() {
            // ---- 1. 处理 GLFW 事件 ----
            self.glfw.poll_events();

            // 检查窗口关闭/ESC 键
            if self.window.get_key(Key::Escape) == Action::Press {
                self.window.set_should_close(true);
            }

            // ---- 2. 接收新帧（只取最新帧，丢弃积压的旧帧） ----
            //
            // 关键优化：用 try_recv 排空通道，但只保留最后收到的一帧进行解码。
            // 这样可以避免对将被覆盖的旧帧做无用的 JPEG 解码（CPU 密集），
            // 是消除播放卡顿的最关键改动。
            let mut latest_jpeg: Option<Vec<u8>> = None;
            while let Ok(jpeg_data) = self.receiver.try_recv() {
                latest_jpeg = Some(jpeg_data); // 旧帧直接丢弃
            }

            if let Some(jpeg_data) = latest_jpeg {
                // 只对最新帧做一次解码
                match decode_jpeg_to_rgba(&jpeg_data) {
                    Ok(frame) => {
                        self.update_texture(&frame.rgba, frame.width, frame.height);
                        has_frame = true;
                    }
                    Err(e) => {
                        eprintln!("[渲染器] 解码帧失败: {}", e);
                    }
                }
            }

            // ---- 3. 渲染 ----
            unsafe {
                // 清屏
                gl::ClearColor(0.05, 0.05, 0.1, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);

                // 使用着色器
                self.shader.use_program();

                if has_frame {
                    // 绑定纹理到纹理单元 0
                    if let Some(ref tex) = self.texture {
                        tex.bind(gl::TEXTURE0);
                        // 设置采样器到纹理单元 0
                        self.shader.set_uniform_1i("ourTexture", 0);
                    }
                }

                // 绘制四边形
                gl::BindVertexArray(self.vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
                gl::BindVertexArray(0);

                // 解绑纹理
                Texture::unbind();
            }

            // ---- 4. 交换缓冲区 ----
            self.window.swap_buffers();

            // ---- 5. 简单帧率控制 ----
            // 避免空转占用 100% CPU
            if !has_frame {
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }

        Ok(())
    }

    /// 更新或创建纹理
    ///
    /// 如果纹理已存在且尺寸相同，使用 glTexSubImage2D 高效更新。
    /// 否则重新创建纹理。
    fn update_texture(&mut self, rgba: &[u8], width: u32, height: u32) {
        match self.texture {
            Some(ref tex) => {
                tex.update(rgba, width, height);
            }
            None => {
                match Texture::from_rgba(rgba, width, height) {
                    Ok(tex) => self.texture = Some(tex),
                    Err(e) => eprintln!("[渲染器] 创建纹理失败: {}", e),
                }
            }
        }
    }

    /// 销毁 OpenGL 资源
    fn cleanup(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
        }
        // texture 和 shader 会在 Drop 中自动释放
    }
}

impl Drop for MjpegRenderer {
    fn drop(&mut self) {
        self.cleanup();
    }
}
