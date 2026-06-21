/// OpenGL 纹理渲染示例
///
/// 本示例演示：
/// 1. 在内存中生成一张彩色测试图片（RGBA 格式）
/// 2. 将图片数据上传到 GPU 作为纹理
/// 3. 使用纹理坐标将图片映射到一个矩形上
/// 4. 为后续播放器开发（YUV/RGB 渲染）打下基础
///
/// 运行方式：
/// ```bash
/// cd src-tauri
/// cargo run --example opengl_texture
/// ```

use gl::types::*;
use glfw::Context;
use std::ffi::CString;

// ======================== 着色器 ========================

/// 顶点着色器
///
/// 现在每个顶点包含两个属性：
/// - aPos: 顶点位置 (location = 0)
/// - aTexCoord: 纹理坐标 (location = 1)
///
/// 纹理坐标范围 (0,0) ~ (1,1)：
/// (0,1) 左上 ──── (1,1) 右上
///   │                 │
///   │   纹理方向       │
///   │                 │
/// (0,0) 左下 ──── (1,0) 右下
const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(aPos.x, aPos.y, aPos.z, 1.0);
    TexCoord = aTexCoord;
}
"#;

/// 片段着色器
///
/// - texture: 采样器，用于从纹理中获取颜色
/// - TexCoord: 从顶点着色器传来的纹理坐标（已插值）
/// - texture() 函数根据坐标从纹理中采样颜色
const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D ourTexture;

void main() {
    FragColor = texture(ourTexture, TexCoord);
}
"#;

fn main() {
    // ======================== 1. 初始化 GLFW ========================
    let mut glfw = glfw::init(glfw::fail_on_errors).expect("GLFW 初始化失败");

    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    // ======================== 2. 创建窗口 ========================
    let (mut window, events) = glfw
        .create_window(
            800,
            600,
            "Learn OpenGL - 纹理渲染",
            glfw::WindowMode::Windowed,
        )
        .expect("窗口创建失败");

    window.make_current();
    window.glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    // ======================== 3. 加载 OpenGL 函数指针 ========================
    gl::load_with(|symbol| {
        let addr = window.get_proc_address(symbol);
        match addr {
            Some(f) => f as *const std::ffi::c_void,
            None => std::ptr::null(),
        }
    });

    unsafe {
        gl::Viewport(0, 0, 800, 600);
    }

    // ======================== 4. 编译着色器 ========================
    let vertex_shader = compile_shader(VERTEX_SHADER_SOURCE, gl::VERTEX_SHADER, "顶点着色器");
    let fragment_shader = compile_shader(FRAGMENT_SHADER_SOURCE, gl::FRAGMENT_SHADER, "片段着色器");
    let shader_program = link_program(vertex_shader, fragment_shader);

    unsafe {
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
    }

    // ======================== 5. 生成测试图片 ========================
    //
    // 生成一张 256x256 的彩色图片，包含：
    // - 四个象限不同颜色（红、绿、蓝、黄）
    // - 中心十字线
    // - 边缘有数字标记，方便观察纹理坐标映射
    //
    // 图片布局：
    //   (红) 左上  │  (绿) 右上
    //   ───────────┼───────────
    //   (蓝) 左下  │  (黄) 右下
    //
    let tex_width = 256;
    let tex_height = 256;
    let test_image = generate_test_pattern(tex_width, tex_height);

    // ======================== 6. 创建 OpenGL 纹理 ========================
    let texture_id = unsafe {
        let mut texture: GLuint = 0;
        gl::GenTextures(1, &mut texture);

        // 绑定纹理（GL_TEXTURE_2D 表示二维纹理）
        gl::BindTexture(gl::TEXTURE_2D, texture);

        // ---- 设置纹理参数 ----

        // 纹理包裹方式（当坐标超出 0~1 范围时）
        // GL_REPEAT: 重复纹理（默认）
        // GL_MIRRORED_REPEAT: 镜像重复
        // GL_CLAMP_TO_EDGE: 边缘拉伸
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);

        // 纹理缩放时的过滤方式
        // GL_LINEAR: 线性插值（平滑）
        // GL_NEAREST: 最近邻采样（锐利，像素风）
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

        // ---- 上传像素数据到 GPU ----
        // 参数说明：
        // - target: 纹理类型 (GL_TEXTURE_2D)
        // - level: 多级渐远纹理层级 (0 = 基础层级)
        // - internalformat: 纹理内部格式 (GL_RGBA)
        // - width, height: 图片尺寸
        // - border: 必须为 0
        // - format: 像素数据格式 (GL_RGBA)
        // - type: 像素数据类型 (GL_UNSIGNED_BYTE)
        // - pixels: 像素数据指针
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as GLint,
            tex_width as GLsizei,
            tex_height as GLsizei,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            test_image.as_ptr() as *const GLvoid,
        );

        // 生成多级渐远纹理（可选，用于提高远处渲染质量）
        gl::GenerateMipmap(gl::TEXTURE_2D);

        // 解绑纹理
        gl::BindTexture(gl::TEXTURE_2D, 0);

        texture
    };

    println!(
        "✅ 纹理已创建，尺寸: {}x{}，像素数: {}",
        tex_width,
        tex_height,
        test_image.len() / 4
    );

    // ======================== 7. 准备顶点数据 ========================
    //
    // 每个顶点现在有 5 个分量：
    //   [x, y, z, u, v]
    //    position  texcoord
    //
    // 纹理坐标 (u, v):
    //   (0, 1) = 左上角, (1, 1) = 右上角
    //   (0, 0) = 左下角, (1, 0) = 右下角
    //
    // 画一个覆盖屏幕大部分区域的矩形，显示纹理

    let vertices: [f32; 30] = [
        // 位置 (x, y, z)    纹理坐标 (u, v)
        // ---- 第一个三角形 ----
        -0.9, 0.9, 0.0, 0.0, 1.0, // 左上
        -0.9, -0.9, 0.0, 0.0, 0.0, // 左下
        0.9, 0.9, 0.0, 1.0, 1.0, // 右上
        // ---- 第二个三角形 ----
        -0.9, -0.9, 0.0, 0.0, 0.0, // 左下
        0.9, -0.9, 0.0, 1.0, 0.0, // 右下
        0.9, 0.9, 0.0, 1.0, 1.0, // 右上
    ];

    // ======================== 8. 创建 VAO & VBO ========================
    let mut vao: GLuint = 0;
    let mut vbo: GLuint = 0;

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
            vertices.as_ptr() as *const GLvoid,
            gl::STATIC_DRAW,
        );

        // 设置顶点位置属性 (location = 0)
        // 每个顶点前 3 个 float 是位置
        let stride = 5 * std::mem::size_of::<f32>() as GLsizei;
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
        gl::EnableVertexAttribArray(0);

        // 设置纹理坐标属性 (location = 1)
        // 每个顶点后 2 个 float 是纹理坐标（偏移 3 个 float）
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

    // ======================== 9. 渲染循环 ========================
    while !window.should_close() {
        process_events(&mut window, &events);

        unsafe {
            // 清屏
            gl::ClearColor(0.15, 0.15, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            // 使用我们的着色器程序
            gl::UseProgram(shader_program);

            // 绑定纹理（将纹理绑定到纹理单元 0）
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);

            // 设置采样器 uniform 指向纹理单元 0
            let tex_location =
                gl::GetUniformLocation(shader_program, CString::new("ourTexture").unwrap().as_ptr());
            gl::Uniform1i(tex_location, 0);

            // 绘制矩形（6 个顶点 = 2 个三角形）
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        window.swap_buffers();
        glfw.poll_events();
    }

    // ======================== 10. 清理资源 ========================
    unsafe {
        gl::DeleteProgram(shader_program);
        gl::DeleteVertexArrays(1, &vao);
        gl::DeleteBuffers(1, &vbo);
        gl::DeleteTextures(1, &texture_id);
    }
}

/// 生成彩色测试图案
///
/// 生成一张 RGBA 格式的测试图片，包含：
/// - 四个象限不同颜色
/// - 中心十字线
/// - 边框标记
///
/// # 参数
/// - `width`: 图片宽度
/// - `height`: 图片高度
///
/// # 返回值
/// RGBA 像素数据（每个像素 4 字节）
fn generate_test_pattern(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let (r, g, b, a) = get_pixel_color(x, y, width, height);
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }

    pixels
}

/// 计算单个像素的颜色
///
/// 使用纹理坐标来确定颜色，方便验证纹理映射是否正确
fn get_pixel_color(x: u32, y: u32, width: u32, height: u32) -> (u8, u8, u8, u8) {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let border = 4.0; // 边框/十字线宽度

    // 判断是否在十字线上（中心十字，宽度 4 像素）
    let on_crosshair = (x as f64 - cx).abs() < border || (y as f64 - cy).abs() < border;
    // 判断是否在边缘边框上
    let on_border = x < border as u32
        || x >= width - border as u32
        || y < border as u32
        || y >= height - border as u32;

    if on_crosshair || on_border {
        // 十字线和边框：白色
        return (255, 255, 255, 255);
    }

    // 四个象限不同颜色
    let left = x < (width / 2);
    let top = y < (height / 2);

    match (top, left) {
        // 左上：红色
        (true, true) => (220, 60, 60, 255),
        // 右上：绿色
        (true, false) => (60, 200, 80, 255),
        // 左下：蓝色
        (false, true) => (60, 80, 220, 255),
        // 右下：黄色
        (false, false) => (220, 200, 40, 255),
    }
}

// ======================== 工具函数 ========================

/// 编译着色器
fn compile_shader(source: &str, shader_type: GLenum, name: &str) -> GLuint {
    let c_source = CString::new(source).expect("CString 创建失败");

    unsafe {
        let shader = gl::CreateShader(shader_type);
        gl::ShaderSource(shader, 1, &c_source.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut success: GLint = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

        if success == 0 {
            let mut info_log_len: GLint = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_log_len);

            let mut info_log = Vec::with_capacity(info_log_len as usize);
            info_log.set_len((info_log_len as usize) - 1);

            gl::GetShaderInfoLog(
                shader,
                info_log_len,
                std::ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );

            let error_msg = String::from_utf8_lossy(&info_log);
            panic!("{} 编译失败:\n{}", name, error_msg);
        }

        shader
    }
}

/// 链接着色器程序
fn link_program(vertex_shader: GLuint, fragment_shader: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);
        gl::LinkProgram(program);

        let mut success: GLint = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);

        if success == 0 {
            let mut info_log_len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut info_log_len);

            let mut info_log = Vec::with_capacity(info_log_len as usize);
            info_log.set_len((info_log_len as usize) - 1);

            gl::GetProgramInfoLog(
                program,
                info_log_len,
                std::ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );

            let error_msg = String::from_utf8_lossy(&info_log);
            panic!("着色器程序链接失败:\n{}", error_msg);
        }

        program
    }
}

/// 处理 GLFW 事件
fn process_events(window: &mut glfw::Window, events: &glfw::GlfwReceiver<(f64, glfw::WindowEvent)>) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            glfw::WindowEvent::Key(glfw::Key::Escape, _, _, _) => {
                window.set_should_close(true);
            }
            glfw::WindowEvent::Size(width, height) => {
                unsafe {
                    gl::Viewport(0, 0, width, height);
                }
            }
            _ => {}
        }
    }
}