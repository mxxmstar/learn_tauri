//! OpenGL 着色器管理模块
//!
//! 提供着色器的编译、链接和统一变量（uniform）设置功能。
//! 封装了 OpenGL 着色器程序的完整生命周期管理。

use gl::types::*;
use std::ffi::CString;

use super::error::RenderError;

/// 纹理渲染专用的顶点着色器
///
/// 输入：
/// - aPos:      顶点位置 (location = 0)，三维坐标 (x, y, z)
/// - aTexCoord: 纹理坐标 (location = 1)，二维坐标 (u, v)
///
/// 输出（到片段着色器）：
/// - TexCoord: 插值后的纹理坐标
pub const VERTEX_SHADER_SRC: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(aPos, 1.0);
    TexCoord = aTexCoord;
}
"#;

/// 纹理渲染专用的片段着色器
///
/// 输入：
/// - TexCoord:   从顶点着色器传来的插值纹理坐标
/// - ourTexture: 纹理采样器（由外部绑定）
///
/// 输出：
/// - FragColor: 最终像素颜色
pub const FRAGMENT_SHADER_SRC: &str = r#"
#version 330 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D ourTexture;

void main() {
    FragColor = texture(ourTexture, TexCoord);
}
"#;

/// YUV→RGB 转换着色器（备用方案，处理原生 YUV 格式的视频帧）
///
/// 如果输入数据为 YUV420P 格式，使用此着色器直接在 GPU 完成色彩空间转换。
/// 相比 CPU 端转换，GPU 转换更高效且不占用主线程。
pub const FRAGMENT_SHADER_YUV_SRC: &str = r#"
#version 330 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D y_tex;
uniform sampler2D u_tex;
uniform sampler2D v_tex;

void main() {
    float y = texture(y_tex, TexCoord).r;
    float u = texture(u_tex, TexCoord).r - 0.5;
    float v = texture(v_tex, TexCoord).r - 0.5;

    float r = y + 1.402 * v;
    float g = y - 0.344 * u - 0.714 * v;
    float b = y + 1.772 * u;

    FragColor = vec4(r, g, b, 1.0);
}
"#;

/// 着色器程序对象
///
/// 封装了 OpenGL 着色器程序的生命周期管理：
/// - 编译顶点着色器 + 片段着色器
/// - 链接为可执行程序
/// - 提供统一的 uniform 设置接口
pub struct ShaderProgram {
    /// OpenGL 程序对象 ID
    id: GLuint,
}

impl ShaderProgram {
    /// 从顶点和片段着色器源码创建着色器程序
    ///
    /// # 参数
    /// - `vertex_src`:   顶点着色器 GLSL 源码
    /// - `fragment_src`: 片段着色器 GLSL 源码
    ///
    /// # 返回值
    /// 编译链接成功的 ShaderProgram 实例
    ///
    /// # 错误
    /// 如果着色器编译或链接失败，返回 RenderError
    pub fn new(vertex_src: &str, fragment_src: &str) -> Result<Self, RenderError> {
        // 编译顶点着色器
        let vs = Self::compile_shader(vertex_src, gl::VERTEX_SHADER, "顶点着色器")?;
        // 编译片段着色器
        let fs = Self::compile_shader(fragment_src, gl::FRAGMENT_SHADER, "片段着色器")?;

        // 链接为程序
        let program = Self::link_program(vs, fs)?;

        // 着色器对象链接后即可释放
        unsafe {
            gl::DeleteShader(vs);
            gl::DeleteShader(fs);
        }

        Ok(ShaderProgram { id: program })
    }

    /// 编译单个着色器
    ///
    /// # 参数
    /// - `source`:       GLSL 源码
    /// - `shader_type`:  着色器类型（gl::VERTEX_SHADER 或 gl::FRAGMENT_SHADER）
    /// - `name`:         着色器名称（用于错误提示）
    fn compile_shader(source: &str, shader_type: GLenum, name: &str) -> Result<GLuint, RenderError> {
        let c_source = CString::new(source).map_err(|_| {
            RenderError::ShaderCompile {
                name: name.to_string(),
                log: "CString 转换失败（源码包含空字节）".to_string(),
            }
        })?;

        unsafe {
            let shader = gl::CreateShader(shader_type);
            gl::ShaderSource(shader, 1, &c_source.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            // 检查编译状态
            let mut success: GLint = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

            if success == 0 {
                // 获取错误信息长度
                let mut info_log_len: GLint = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut info_log_len);

                let mut info_log = Vec::with_capacity(info_log_len as usize);
                info_log.set_len((info_log_len as usize).saturating_sub(1));

                gl::GetShaderInfoLog(
                    shader,
                    info_log_len,
                    std::ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );

                let log = String::from_utf8_lossy(&info_log).to_string();
                gl::DeleteShader(shader);
                return Err(RenderError::ShaderCompile {
                    name: name.to_string(),
                    log,
                });
            }

            Ok(shader)
        }
    }

    /// 链接着色器程序
    ///
    /// # 参数
    /// - `vertex_shader`:   编译好的顶点着色器 ID
    /// - `fragment_shader`: 编译好的片段着色器 ID
    fn link_program(vertex_shader: GLuint, fragment_shader: GLuint) -> Result<GLuint, RenderError> {
        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            // 检查链接状态
            let mut success: GLint = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);

            if success == 0 {
                let mut info_log_len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut info_log_len);

                let mut info_log = Vec::with_capacity(info_log_len as usize);
                info_log.set_len((info_log_len as usize).saturating_sub(1));

                gl::GetProgramInfoLog(
                    program,
                    info_log_len,
                    std::ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );

                let log = String::from_utf8_lossy(&info_log).to_string();
                gl::DeleteProgram(program);
                return Err(RenderError::ProgramLink { log });
            }

            Ok(program)
        }
    }

    /// 激活着色器程序
    ///
    /// 调用 glUseProgram 使此着色器程序成为当前渲染状态的一部分
    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    /// 获取 uniform 变量的位置
    ///
    /// # 参数
    /// - `name`: uniform 变量名
    ///
    /// # 返回值
    /// uniform 位置，如果变量不存在或未使用则返回 -1
    pub fn get_uniform_location(&self, name: &str) -> GLint {
        let c_name = CString::new(name).expect("uniform 名包含空字节");
        unsafe { gl::GetUniformLocation(self.id, c_name.as_ptr()) }
    }

    /// 设置 1 个整数的 uniform 变量
    pub fn set_uniform_1i(&self, name: &str, v0: GLint) {
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            unsafe { gl::Uniform1i(loc, v0) };
        }
    }

    /// 设置 4 个浮点数的 uniform 变量（常用于颜色/偏移量）
    pub fn set_uniform_4f(&self, name: &str, v0: GLfloat, v1: GLfloat, v2: GLfloat, v3: GLfloat) {
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            unsafe { gl::Uniform4f(loc, v0, v1, v2, v3) };
        }
    }

    /// 获取 OpenGL 程序 ID
    pub fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for ShaderProgram {
    /// 析构时自动释放 OpenGL 着色器程序资源
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
