//! FFI 接口定义
//!
//! 对接 C++/FFmpeg 的 FFI 接口
//!
//! # C++ 端接口设计
//!
//! C++ 端需要提供以下 C 风格接口：
//!
//! ```cpp
//! // 创建解码器
//! DecoderHandle decoder_create(int codec_type, int pixel_format);
//!
//! // 销毁解码器
//! void decoder_destroy(DecoderHandle handle);
//!
//! // 解码一帧
//! int decoder_decode(DecoderHandle handle, const uint8_t* data, size_t size,
//!                    int64_t pts, int64_t dts, int keyframe);
//!
//! // 获取解码后的帧
//! int decoder_get_frame(DecoderHandle handle, DecodedFrame* frame);
//!
//! // 刷新解码器
//! int decoder_flush(DecoderHandle handle);
//!
//! // 重置解码器
//! void decoder_reset(DecoderHandle handle);
//! ```
//!
//! # Feature Flags
//! - `decoder-ffi`: 启用 FFI 实现

use std::ffi::c_void;

/// 解码器句柄（C++ 端的解码器实例指针）
pub type DecoderHandle = *mut c_void;

/// 解码后帧的 C 结构（与 C++ 端对齐）
#[repr(C)]
pub struct DecodedFrame {
    /// 像素格式
    pub pixel_format: i32,
    /// 宽度
    pub width: i32,
    /// 高度
    pub height: i32,
    /// 数据指针
    pub data: *const u8,
    /// 数据大小
    pub size: usize,
    /// 时间戳
    pub pts: i64,
    pub dts: i64,
    /// 是否为关键帧
    pub keyframe: bool,
}

/// FFI 错误码
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiErrorCode {
    /// 成功
    Success = 0,
    /// 失败
    Failed = -1,
    /// 无效参数
    InvalidParameter = -2,
    /// 解码错误
    DecodeError = -3,
    /// 缓冲区满
    BufferFull = -4,
    /// 超时
    Timeout = -5,
}

/// FFI 解码器包装器
///
/// 用于包装 C++ 端的解码器实例
pub struct FfiDecoder {
    /// 解码器句柄
    handle: DecoderHandle,
    /// 编解码器类型
    codec_type: i32,
    /// 输出像素格式
    pixel_format: i32,
}

impl FfiDecoder {
    /// 创建新的 FFI 解码器
    pub fn new(codec_type: i32, pixel_format: i32) -> Result<Self, String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_create 函数
            // let handle = unsafe { ffi::decoder_create(codec_type, pixel_format) };
            // if handle.is_null() {
            //     return Err("failed to create decoder".to_string());
            // }
            // Ok(Self { handle, codec_type, pixel_format })
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 解码数据
    pub fn decode(
        &mut self,
        data: &[u8],
        pts: i64,
        dts: i64,
        keyframe: bool,
    ) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_decode 函数
            // let ret = unsafe {
            //     ffi::decoder_decode(
            //         self.handle,
            //         data.as_ptr(),
            //         data.len(),
            //         pts,
            //         dts,
            //         keyframe,
            //     )
            // };
            // if ret != 0 {
            //     return Err(format!("decode failed: {}", ret));
            // }
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 获取解码后的帧
    pub fn get_frame(&mut self) -> Result<Option<DecodedFrame>, String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_get_frame 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 刷新解码器
    pub fn flush(&mut self) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_flush 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }

    /// 重置解码器
    pub fn reset(&mut self) -> Result<(), String> {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_reset 函数
            return Err("FFI decoder not implemented yet".to_string());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err("FFI decoder requires 'decoder-ffi' feature".to_string())
        }
    }
}

impl Drop for FfiDecoder {
    fn drop(&mut self) {
        #[cfg(feature = "decoder-ffi")]
        {
            // TODO: 调用 C++ 端的 decoder_destroy 函数
            // if !self.handle.is_null() {
            //     unsafe { ffi::decoder_destroy(self.handle) };
            // }
        }
    }
}

/// 声明外部 C 函数（C++ 端实现）
#[cfg(feature = "decoder-ffi")]
#[allow(non_snake_case)]
mod ffi {
    use super::*;

    extern "C" {
        /// 创建解码器
        pub fn decoder_create(codec_type: i32, pixel_format: i32) -> DecoderHandle;

        /// 销毁解码器
        pub fn decoder_destroy(handle: DecoderHandle);

        /// 解码一帧
        pub fn decoder_decode(
            handle: DecoderHandle,
            data: *const u8,
            size: usize,
            pts: i64,
            dts: i64,
            keyframe: bool,
        ) -> i32;

        /// 获取解码后的帧
        pub fn decoder_get_frame(handle: DecoderHandle, frame: *mut DecodedFrame) -> i32;

        /// 刷新解码器
        pub fn decoder_flush(handle: DecoderHandle) -> i32;

        /// 重置解码器
        pub fn decoder_reset(handle: DecoderHandle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_decoder_new() {
        // 默认应该返回错误（未实现或 feature 未启用）
        let result = FfiDecoder::new(0, 1);
        assert!(result.is_err());
    }
}
