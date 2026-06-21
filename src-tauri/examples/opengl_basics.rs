/// OpenGL 基础渲染示例
///
/// 本示例演示如何使用 glfw + gl 在 Rust 中：
/// 1. 创建 OpenGL 窗口
/// 2. 编写并使用着色器（Vertex Shader + Fragment Shader）
/// 3. 渲染一个三角形
/// 4. 渲染一个矩形（由两个三角形组成）
///
/// 运行方式：
/// ```bash
/// cd src-tauri
/// cargo run --example opengl_basics
/// ```

use gl::types::*;
use glfw::Context;
use std::ffi::CString;

/// 顶点着色器源码（GLSL）
///
/// 作用：处理每个顶点的位置，将 3D 坐标转换为屏幕坐标
/// - `layout (location = 0)` 表示从顶点属性数组的第 0 号位置读取数据
/// - `vec3` 是三维向量，对应顶点的 (x, y, z) 坐标
/// - `gl_Position` 是内置变量，表示顶点在屏幕上的最终位置
const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
void main() {
    gl_Position = vec4(aPos.x, aPos.y, aPos.z, 1.0);
}
"#;

/// 片段着色器源码（GLSL）
///
/// 作用：计算每个像素（片段）的最终颜色
/// - `uniform vec4 ourColor` 是由外部传入的颜色变量（可以在绘制前修改）
/// - 使用 uniform 可以让同一个着色器程序绘制不同颜色
const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core
out vec4 FragColor;
uniform vec4 ourColor;
void main() {
    FragColor = ourColor;
}
"#;

fn main() {
    // ======================== 1. 初始化 GLFW ========================
    let mut glfw = glfw::init(glfw::fail_on_errors).expect("GLFW 初始化失败");

    // 设置 OpenGL 版本为 3.3
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    // 使用 Core Profile（核心模式），弃用旧版 OpenGL
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    // macOS 需要 Forward Compat
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    // ======================== 2. 创建窗口 ========================
    let (mut window, events) = glfw
        .create_window(800, 600, "Learn OpenGL - 三角形 & 矩形", glfw::WindowMode::Windowed)
        .expect("窗口创建失败");

    // 设置当前 OpenGL 上下文
    window.make_current();
    // 启用垂直同步（VSync）
    window.glfw.set_swap_interval(glfw::SwapInterval::Sync(1));

    // ======================== 3. 加载 OpenGL 函数指针 ========================
    gl::load_with(|symbol| {
        let addr = window.get_proc_address(symbol);
        match addr {
            Some(f) => f as *const std::ffi::c_void,
            None => std::ptr::null(),
        }
    });

    // 设置视口大小（与窗口大小一致）
    unsafe {
        gl::Viewport(0, 0, 800, 600);
    }

    // ======================== 4. 编译着色器 ========================

    // --- 4a. 编译顶点着色器 ---
    let vertex_shader = compile_shader(
        VERTEX_SHADER_SOURCE,
        gl::VERTEX_SHADER,
        "顶点着色器",
    );

    // --- 4b. 编译片段着色器 ---
    let fragment_shader = compile_shader(
        FRAGMENT_SHADER_SOURCE,
        gl::FRAGMENT_SHADER,
        "片段着色器",
    );

    // --- 4c. 链接着色器程序 ---
    let shader_program = link_program(vertex_shader, fragment_shader);

    // 着色器链接完成后可以删除中间产物
    unsafe {
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
    }

    // ======================== 5. 准备顶点数据 ========================

    // --- 5a. 三角形的三个顶点（NDC 坐标系，范围 -1 ~ 1）---
    // 一个等腰三角形，在屏幕左侧
    let triangle_vertices: [f32; 9] = [
        // 位置 (x, y, z)
        -0.5, 0.5, 0.0,  // 顶点
        -0.9, -0.5, 0.0, // 左下
        -0.1, -0.5, 0.0, // 右下
    ];

    // --- 5b. 矩形的六个顶点（两个三角形组成一个矩形）---
    // 在屏幕右侧，由两个三角形拼成
    //
    //   0───2
    //   │ ╱ │
    //   1───3
    //
    // 三角形1: 0 -> 1 -> 2
    // 三角形2: 1 -> 3 -> 2
    let rectangle_vertices: [f32; 18] = [
        // 第一个三角形
        0.1, 0.5, 0.0,   // 左上 (0)
        0.1, -0.5, 0.0,  // 左下 (1)
        0.9, 0.5, 0.0,   // 右上 (2)
        // 第二个三角形
        0.1, -0.5, 0.0,  // 左下 (1)
        0.9, -0.5, 0.0,  // 右下 (3)
        0.9, 0.5, 0.0,   // 右上 (2)
    ];

    // ======================== 6. 创建 VAO & VBO ========================

    // --- 6a. 三角形的 VAO 和 VBO ---
    let (triangle_vao, triangle_vbo) = create_vertex_buffer(&triangle_vertices);

    // --- 6b. 矩形的 VAO 和 VBO ---
    let (rectangle_vao, rectangle_vbo) = create_vertex_buffer(&rectangle_vertices);

    // ======================== 7. 渲染循环 ========================
    while !window.should_close() {
        // 处理事件（窗口缩放、键盘输入等）
        process_events(&mut window, &events);

        // --- 7a. 清屏 ---
        // 设置清除颜色为深蓝灰色
        unsafe {
            gl::ClearColor(0.2, 0.2, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        // --- 7b. 绘制三角形（左侧）---
        unsafe {
            gl::UseProgram(shader_program);

            // 获取 uniform 变量的位置
            let color_location = gl::GetUniformLocation(shader_program, CString::new("ourColor").unwrap().as_ptr());
            // 设置颜色：橙色 (1.0, 0.5, 0.2, 1.0)
            gl::Uniform4f(color_location, 1.0, 0.5, 0.2, 1.0);

            gl::BindVertexArray(triangle_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
        }

        // --- 7c. 绘制矩形（右侧）---
        unsafe {
            gl::UseProgram(shader_program);

            // 设置颜色：青绿色 (0.2, 0.8, 0.6, 1.0)
            let color_location = gl::GetUniformLocation(shader_program, CString::new("ourColor").unwrap().as_ptr());
            gl::Uniform4f(color_location, 0.2, 0.8, 0.6, 1.0);

            gl::BindVertexArray(rectangle_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        // --- 7d. 交换缓冲区 ---
        window.swap_buffers();
        // 等待下一帧（与 VSync 配合）
        glfw.poll_events();
    }

    // ======================== 8. 清理资源 ========================
    unsafe {
        gl::DeleteProgram(shader_program);
        gl::DeleteVertexArrays(1, &triangle_vao);
        gl::DeleteBuffers(1, &triangle_vbo);
        gl::DeleteVertexArrays(1, &rectangle_vao);
        gl::DeleteBuffers(1, &rectangle_vbo);
    }
}

/// 编译着色器
///
/// # 参数
/// - `source`: GLSL 着色器源码
/// - `shader_type`: 着色器类型（`gl::VERTEX_SHADER` 或 `gl::FRAGMENT_SHADER`）
/// - `name`: 着色器名称（用于错误提示）
///
/// # 返回值
/// 编译成功的着色器对象 ID
fn compile_shader(source: &str, shader_type: GLenum, name: &str) -> GLuint {
    // 将 Rust 字符串转为 C 字符串
    let c_source = CString::new(source).expect("CString 创建失败");

    unsafe {
        // 创建着色器对象
        let shader = gl::CreateShader(shader_type);

        // 传递源码给 OpenGL
        gl::ShaderSource(shader, 1, &c_source.as_ptr(), std::ptr::null());

        // 编译着色器
        gl::CompileShader(shader);

        // 检查编译结果
        let mut success: GLint = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

        if success == 0 {
            // 获取错误信息
            let mut info_log_len: GLint = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_log_len);

            let mut info_log = Vec::with_capacity(info_log_len as usize);
            info_log.set_len((info_log_len as usize) - 1); // 去掉 null 终止符

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
///
/// # 参数
/// - `vertex_shader`: 编译好的顶点着色器
/// - `fragment_shader`: 编译好的片段着色器
///
/// # 返回值
/// 链接成功的着色器程序对象 ID
fn link_program(vertex_shader: GLuint, fragment_shader: GLuint) -> GLuint {
    unsafe {
        // 创建着色器程序
        let program = gl::CreateProgram();

        // 附着着色器
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);

        // 链接
        gl::LinkProgram(program);

        // 检查链接结果
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

/// 创建顶点缓冲区
///
/// 封装了 VAO（顶点数组对象）和 VBO（顶点缓冲对象）的创建流程：
/// 1. 生成 VAO 并绑定
/// 2. 生成 VBO 并绑定
/// 3. 将顶点数据上传到 GPU
/// 4. 设置顶点属性指针（告诉 GPU 如何解析数据）
///
/// # 参数
/// - `vertices`: 顶点数据数组
///
/// # 返回值
/// (vao, vbo) 的元组
fn create_vertex_buffer(vertices: &[f32]) -> (GLuint, GLuint) {
    let mut vao: GLuint = 0;
    let mut vbo: GLuint = 0;

    unsafe {
        // 生成并绑定 VAO
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // 生成并绑定 VBO
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        // 将顶点数据上传到 GPU
        // gl::STATIC_DRAW 表示数据不会频繁改变
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
            vertices.as_ptr() as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );

        // 设置顶点属性指针
        // - index = 0: 对应着色器中的 `layout (location = 0)`
        // - size = 3: 每个顶点有 3 个分量 (x, y, z)
        // - type = FLOAT: 数据类型为浮点数
        // - normalized = FALSE: 不归一化
        // - stride = 3 * sizeof(float): 每个顶点的间隔（步长）
        // - pointer = 0: 从数据的起始位置开始读取
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<f32>() as GLsizei, std::ptr::null());
        gl::EnableVertexAttribArray(0);

        // 解绑 VBO（VAO 会记住这个设置）
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        // 解绑 VAO
        gl::BindVertexArray(0);
    }

    (vao, vbo)
}

/// 处理 GLFW 事件
fn process_events(window: &mut glfw::Window, events: &glfw::GlfwReceiver<(f64, glfw::WindowEvent)>) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            // 按 ESC 键关闭窗口
            glfw::WindowEvent::Key(glfw::Key::Escape, _, _, _) => {
                window.set_should_close(true);
            }
            // 窗口大小变化时更新视口
            glfw::WindowEvent::Size(width, height) => {
                unsafe {
                    gl::Viewport(0, 0, width, height);
                }
            }
            _ => {}
        }
    }
}