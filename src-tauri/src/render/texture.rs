//! OpenGL 纹理管理模块
//!
//! 提供纹理的创建、更新和销毁功能。
//! 支持 RGBA 格式的像素数据上传到 GPU 纹理。
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use learn_tauri_lib::render::texture::Texture;
//!
//! // 从 RGBA 像素数据创建纹理
//! let rgba_data = vec![255u8, 0, 0, 255]; // 一个红色像素
//! let texture = Texture::from_rgba(&rgba_data, 1, 1).expect("纹理创建失败");
//!
//! // 更新纹理数据（用于视频帧更新）
//! texture.update(&new_rgba_data, new_width, new_height);
//!
//! // 绑定到纹理单元
//! texture.bind(gl::TEXTURE0);
//! ```

use gl::types::*;
use super::error::RenderError;

/// OpenGL 纹理对象
///
/// 封装了 2D 纹理的完整生命周期管理：
/// - 创建纹理并上传像素数据
/// - 更新纹理数据（替换整个图像）
/// - 自动释放 GPU 资源
pub struct Texture {
    /// OpenGL 纹理对象 ID
    id: GLuint,
    /// 纹理宽度（像素）
    width: u32,
    /// 纹理高度（像素）
    height: u32,
}

impl Texture {
    /// 从 RGBA 像素数据创建新的 2D 纹理
    ///
    /// # 参数
    /// - `data`:   RGBA 格式的像素数据，长度应为 width * height * 4
    /// - `width`:  纹理宽度（像素）
    /// - `height`: 纹理高度（像素）
    ///
    /// # 返回值
    /// 创建成功的 Texture 实例
    ///
    /// # 错误
    /// 如果数据长度与尺寸不匹配，返回 RenderError
    pub fn from_rgba(data: &[u8], width: u32, height: u32) -> Result<Self, RenderError> {
        let expected_len = (width * height * 4) as usize;
        if data.len() != expected_len {
            return Err(RenderError::TextureError(format!(
                "像素数据长度不匹配：期望 {} 字节，实际 {} 字节",
                expected_len,
                data.len()
            )));
        }

        let mut id: GLuint = 0;

        unsafe {
            // 生成纹理对象
            gl::GenTextures(1, &mut id);
            // 绑定为 2D 纹理
            gl::BindTexture(gl::TEXTURE_2D, id);

            // ---- 设置纹理包裹方式 ----
            // S/T 方向都使用 CLAMP_TO_EDGE，避免视频边缘出现重复/镜像伪影
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);

            // ---- 设置纹理过滤方式 ----
            // 缩小使用线性过滤（防止锯齿）
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            // 放大使用线性过滤（平滑放大）
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

            // ---- 上传像素数据到 GPU ----
            gl::TexImage2D(
                gl::TEXTURE_2D,     // 目标纹理类型
                0,                  // 多级渐远纹理层级（0 = 基础层级）
                gl::RGBA as GLint,  // 内部格式（GPU 存储格式）
                width as GLsizei,   // 纹理宽度
                height as GLsizei,  // 纹理高度
                0,                  // 边框（必须为 0）
                gl::RGBA,           // 像素数据格式
                gl::UNSIGNED_BYTE,  // 像素数据类型
                data.as_ptr() as *const GLvoid, // 像素数据指针
            );

            // 生成多级渐远纹理（提高远处渲染质量，视频播放中通常不需要）
            // 但生成 mipmap 会导致额外开销，视频帧更新频繁时建议跳过
            // gl::GenerateMipmap(gl::TEXTURE_2D);

            // 解绑纹理
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        Ok(Texture { id, width, height })
    }

    /// 更新纹理数据（用于视频帧更新）
    ///
    /// 使用 glTexSubImage2D 替换纹理的全部像素数据。
    /// 比重新创建纹理更高效，避免了重新分配 GPU 内存。
    ///
    /// # 参数
    /// - `data`:   新的 RGBA 像素数据
    /// - `width`:  新纹理宽度（像素）
    /// - `height`: 新纹理高度（像素）
    ///
    /// # 说明
    /// 如果新尺寸与原纹理尺寸不同，会重新分配纹理存储。
    /// 更新频繁的视频帧建议保持尺寸一致以获得最佳性能。
    pub fn update(&self, data: &[u8], width: u32, height: u32) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);

            // 如果尺寸变化，需要重新分配纹理存储
            if width != self.width || height != self.height {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as GLint,
                    width as GLsizei,
                    height as GLsizei,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as *const GLvoid,
                );
            } else {
                // 尺寸相同，使用更高效的局部更新
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width as GLsizei,
                    height as GLsizei,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as *const GLvoid,
                );
            }

            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    /// 将纹理绑定到指定的纹理单元
    ///
    /// 在绘制前调用，将纹理绑定到着色器可以访问的纹理单元。
    ///
    /// # 参数
    /// - `unit`: 纹理单元，如 gl::TEXTURE0、gl::TEXTURE1 等
    pub fn bind(&self, unit: GLenum) {
        unsafe {
            gl::ActiveTexture(unit);
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    /// 解绑纹理
    pub fn unbind() {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    /// 获取纹理宽度
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 获取纹理高度
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 获取 OpenGL 纹理对象 ID
    pub fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for Texture {
    /// 析构时自动删除 OpenGL 纹理对象
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}
