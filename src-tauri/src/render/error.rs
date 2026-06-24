//! 渲染模块错误类型定义
//!
//! 定义了渲染模块中可能出现的所有错误：
//! - OpenGL 相关错误（着色器编译、程序链接等）
//! - MJPEG 解析错误（无效的帧标记、解码失败等）
//! - 纹理操作错误（更新失败等）

use thiserror::Error;

/// 渲染模块通用错误类型
#[derive(Error, Debug)]
pub enum RenderError {
    /// GLFW 初始化失败
    #[error("GLFW 初始化失败: {0}")]
    GlfwInit(String),

    /// 窗口创建失败
    #[error("OpenGL 窗口创建失败: {0}")]
    WindowCreate(String),

    /// 着色器编译失败
    #[error("着色器编译失败 ({name}): {log}")]
    ShaderCompile {
        /// 着色器名称（用于标识）
        name: String,
        /// OpenGL 返回的编译日志
        log: String,
    },

    /// 着色器程序链接失败
    #[error("着色器程序链接失败: {log}")]
    ProgramLink {
        /// OpenGL 返回的链接日志
        log: String,
    },

    /// 未找到起始标记（SOI 0xFFD8）
    #[error("未找到 JPEG SOI 标记")]
    MissingSoi,

    /// 未找到结束标记（EOI 0xFFD9）
    #[error("未找到 JPEG EOI 标记")]
    MissingEoi,

    /// JPEG 解码失败
    #[error("JPEG 解码失败: {0}")]
    JpegDecode(String),

    /// 图片格式转换失败
    #[error("图片格式转换失败: {0}")]
    ImageConvert(String),

    /// 纹理操作失败
    #[error("纹理操作失败: {0}")]
    TextureError(String),

    /// 通道接收失败（渲染线程可能已退出）
    #[error("渲染通道错误: {0}")]
    ChannelError(String),

    /// 线程 Join 失败
    #[error("线程 Join 失败")]
    ThreadJoin,
}
