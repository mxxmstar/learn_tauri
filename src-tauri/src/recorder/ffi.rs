//! 录像模块 FFI 接口定义
//!
//! 对接 C++/FFmpeg 进行视频封装（MP4/AVI）
//!
//! # C++ 端接口设计
//!
//! C++ 端需要提供以下 C 风格接口：
//!
//! ```cpp
//! // 创建录像器
//! RecorderHandle recorder_create(int codec_type, int container_format,
//!                             const char* output_path);
//!
//! // 销毁录像器
//! void recorder_destroy(RecorderHandle handle);
//!
//! // 开始录像
//! int recorder_start(RecorderHandle handle, int width, int height, double framerate);
//!
//! // 写入视频帧
//! int recorder_write_frame(RecorderHandle handle, const uint8_t* data, size_t size,
//!                        int64_t timestamp_ms, int keyframe);
//!
//! // 结束录像
//! int recorder_finish(RecorderHandle handle);
//!
//! // 取消录像
//! int recorder_cancel(RecorderHandle handle);
//!
//! // 获取录像统计信息
//! int recorder_get_stats(RecorderHandle handle, RecorderStats* stats);
//! ```
//!
//! # Feature Flags
//! - `recorder-ffi`: 启用 FFI 实现

use std::ffi::c_void;
use std::os::raw::c_char;

use super::error::{RecordError, RecordResult};
use super::config::RecorderConfig;
use super::trait_::Recorder;
use super::trait_::RecordStats;

/// 录像器句柄（C++ 端的录像器实例指针）
pub type RecorderHandle = *mut c_void;

/// 录像统计信息（C 兼容结构）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RecorderStatsFFI {
    /// 开始时间戳（Unix 毫秒）
    pub start_timestamp_ms: u64,
    /// 结束时间戳（Unix 毫秒）
    pub end_timestamp_ms: u64,
    /// 写入的帧数
    pub frames_written: u64,
    /// 写入的字节数
    pub bytes_written: u64,
    /// 录像持续时间（毫秒）
    pub duration_ms: u64,
}

impl Default for RecorderStatsFFI {
    fn default() -> Self {
        Self {
            start_timestamp_ms: 0,
            end_timestamp_ms: 0,
            frames_written: 0,
            bytes_written: 0,
            duration_ms: 0,
        }
    }
}

/// FFI 录像器包装器
///
/// 用于包装 C++ 端的录像器实例
pub struct FfiRecorder {
    /// 录像器句柄
    handle: RecorderHandle,
    /// 是否已开始录像
    started: bool,
    /// 是否已结束录像
    finished: bool,
    /// 录像配置
    config: Option<RecorderConfig>,
}

// FfiRecorder 包含裸指针，默认不实现 Send。
// 由于 C++ 端的录像器实例可以跨线程使用（内部加锁），这里手动声明 Send。
// SAFETY: C++ 端实现保证线程安全
unsafe impl Send for FfiRecorder {}

impl FfiRecorder {
    /// 创建新的 FFI 录像器
    ///
    /// # 参数
    /// * `codec_type` - 编解码器类型（对应 CodecType）
    /// * `container_format` - 容器格式（对应 ContainerFormat）
    /// * `output_path` - 输出文件路径
    ///
    /// # 返回
    /// * `Ok(FfiRecorder)` - 成功创建
    /// * `Err(String)` - 创建失败
    pub fn new(codec_type: i32, container_format: i32, output_path: &str) -> Result<Self, String> {
        #[cfg(feature = "recorder-ffi")]
        {
            let output_path_cstr = std::ffi::CString::new(output_path)
                .map_err(|e| format!("Invalid output path: {}", e))?;

            let handle = unsafe {
                ffi::recorder_create(
                    codec_type,
                    container_format,
                    output_path_cstr.as_ptr(),
                )
            };

            if handle.is_null() {
                return Err("Failed to create recorder".to_string());
            }

            Ok(Self {
                handle,
                started: false,
                finished: false,
                config: None,
            })
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            let _ = (codec_type, container_format, output_path);
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 开始录像
    ///
    /// # 参数
    /// * `width` - 视频宽度（可选）
    /// * `height` - 视频高度（可选）
    /// * `framerate` - 帧率（可选）
    ///
    /// # 返回
    /// * `Ok(())` - 成功开始
    /// * `Err(String)` - 开始失败
    pub fn start(&mut self, width: Option<u32>, height: Option<u32>, framerate: Option<f64>) -> Result<(), String> {
        #[cfg(feature = "recorder-ffi")]
        {
            if self.handle.is_null() {
                return Err("Recorder handle is null".to_string());
            }

            if self.started {
                return Err("Recorder already started".to_string());
            }

            let width = width.unwrap_or(0) as i32;
            let height = height.unwrap_or(0) as i32;
            let framerate = framerate.unwrap_or(0.0);

            let ret = unsafe {
                ffi::recorder_start(self.handle, width, height, framerate)
            };

            if ret != 0 {
                return Err(format!("Failed to start recorder: error code {}", ret));
            }

            self.started = true;
            self.finished = false;
            Ok(())
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 写入视频帧
    ///
    /// # 参数
    /// * `data` - 视频帧数据
    /// * `timestamp_ms` - 时间戳（毫秒，可选）
    /// * `keyframe` - 是否为关键帧
    ///
    /// # 返回
    /// * `Ok(())` - 成功写入
    /// * `Err(String)` - 写入失败
    pub fn write_frame(
        &mut self,
        data: &[u8],
        timestamp_ms: Option<u64>,
        keyframe: bool,
    ) -> Result<(), String> {
        #[cfg(feature = "recorder-ffi")]
        {
            if self.handle.is_null() {
                return Err("Recorder handle is null".to_string());
            }

            if !self.started {
                return Err("Recorder not started".to_string());
            }

            if self.finished {
                return Err("Recorder already finished".to_string());
            }

            let timestamp_ms = timestamp_ms.unwrap_or(0) as i64;
            let keyframe = if keyframe { 1 } else { 0 };

            let ret = unsafe {
                ffi::recorder_write_frame(
                    self.handle,
                    data.as_ptr(),
                    data.len(),
                    timestamp_ms,
                    keyframe,
                )
            };

            if ret != 0 {
                return Err(format!("Failed to write frame: error code {}", ret));
            }

            Ok(())
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 结束录像
    ///
    /// # 返回
    /// * `Ok(())` - 成功结束
    /// * `Err(String)` - 结束失败
    pub fn finish(&mut self) -> Result<(), String> {
        #[cfg(feature = "recorder-ffi")]
        {
            if self.handle.is_null() {
                return Err("Recorder handle is null".to_string());
            }

            if !self.started {
                return Err("Recorder not started".to_string());
            }

            if self.finished {
                return Ok(()); // 已经结束，返回成功
            }

            let ret = unsafe { ffi::recorder_finish(self.handle) };

            if ret != 0 {
                return Err(format!("Failed to finish recorder: error code {}", ret));
            }

            self.finished = true;
            Ok(())
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 取消录像（不保存文件）
    ///
    /// # 返回
    /// * `Ok(())` - 成功取消
    /// * `Err(String)` - 取消失败
    pub fn cancel(&mut self) -> Result<(), String> {
        #[cfg(feature = "recorder-ffi")]
        {
            if self.handle.is_null() {
                return Err("Recorder handle is null".to_string());
            }

            let ret = unsafe { ffi::recorder_cancel(self.handle) };

            if ret != 0 {
                return Err(format!("Failed to cancel recorder: error code {}", ret));
            }

            self.finished = true;
            Ok(())
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 获取录像统计信息
    ///
    /// # 返回
    /// * `Ok(RecorderStatsFFI)` - 统计信息
    /// * `Err(String)` - 获取失败
    pub fn get_stats(&self) -> Result<RecorderStatsFFI, String> {
        #[cfg(feature = "recorder-ffi")]
        {
            if self.handle.is_null() {
                return Err("Recorder handle is null".to_string());
            }

            let mut stats = RecorderStatsFFI::default();
            let ret = unsafe { ffi::recorder_get_stats(self.handle, &mut stats) };

            if ret != 0 {
                return Err(format!("Failed to get stats: error code {}", ret));
            }

            Ok(stats)
        }

        #[cfg(not(feature = "recorder-ffi"))]
        {
            Err("FFI recorder requires 'recorder-ffi' feature".to_string())
        }
    }

    /// 检查是否正在录像
    pub fn is_recording(&self) -> bool {
        self.started && !self.finished
    }

    /// 获取 FFI 句柄（用于高级用法）
    pub fn handle(&self) -> RecorderHandle {
        self.handle
    }
}

/// 为 FfiRecorder 实现 Recorder trait
impl Recorder for FfiRecorder {
    fn start(&mut self, config: RecorderConfig) -> RecordResult<()> {
        if self.started {
            return Err(RecordError::AlreadyFinished);
        }

        // 存储配置
        self.config = Some(config.clone());

        // 调用 FFI start
        self.start(config.width, config.height, config.framerate)
            .map_err(RecordError::InitError)
    }

    fn write_frame(&mut self, frame: &[u8], timestamp_ms: Option<u64>) -> RecordResult<()> {
        // 对于原始字节接口，无法判断关键帧，默认为非关键帧
        self.write_frame(frame, timestamp_ms, false)
            .map_err(|e| RecordError::WriteError(e))
    }

    fn write_media_frame(&mut self, frame: &crate::rtp::decoder::frame::MediaFrame) -> RecordResult<()> {
        // pts 是微秒，转换为毫秒
        let timestamp_ms = if frame.pts >= 0 {
            Some((frame.pts / 1000) as u64)
        } else {
            None
        };
        self.write_frame(&frame.data, timestamp_ms, frame.keyframe)
            .map_err(|e| RecordError::WriteError(e))
    }

    fn finish(&mut self) -> RecordResult<()> {
        self.finish()
            .map_err(|e| RecordError::WriteError(e))
    }

    fn get_stats(&self) -> RecordStats {
        let ffi_stats = self.get_stats()
            .unwrap_or(RecorderStatsFFI::default());
        
        RecordStats {
            start_time: if ffi_stats.start_timestamp_ms > 0 {
                Some(std::time::UNIX_EPOCH + std::time::Duration::from_millis(ffi_stats.start_timestamp_ms))
            } else {
                None
            },
            end_time: if ffi_stats.end_timestamp_ms > 0 {
                Some(std::time::UNIX_EPOCH + std::time::Duration::from_millis(ffi_stats.end_timestamp_ms))
            } else {
                None
            },
            start_timestamp_ms: if ffi_stats.start_timestamp_ms > 0 {
                Some(ffi_stats.start_timestamp_ms)
            } else {
                None
            },
            end_timestamp_ms: if ffi_stats.end_timestamp_ms > 0 {
                Some(ffi_stats.end_timestamp_ms)
            } else {
                None
            },
            frames_written: ffi_stats.frames_written,
            bytes_written: ffi_stats.bytes_written,
            duration_ms: if ffi_stats.duration_ms > 0 {
                Some(ffi_stats.duration_ms)
            } else {
                None
            },
        }
    }

    fn is_recording(&self) -> bool {
        self.is_recording()
    }

    fn get_config(&self) -> Option<&RecorderConfig> {
        self.config.as_ref()
    }

    fn cancel(&mut self) -> RecordResult<()> {
        self.cancel()
            .map_err(|e| RecordError::WriteError(e))
    }
}

impl Drop for FfiRecorder {
    fn drop(&mut self) {
        #[cfg(feature = "recorder-ffi")]
        {
            if !self.handle.is_null() {
                // 如果还在录像，先结束
                if self.started && !self.finished {
                    let _ = unsafe { ffi::recorder_finish(self.handle) };
                }
                // 销毁录像器
                unsafe { ffi::recorder_destroy(self.handle) };
                self.handle = std::ptr::null_mut();
            }
        }
    }
}

/// 声明外部 C 函数（C++ 端实现）
#[cfg(feature = "recorder-ffi")]
#[allow(non_snake_case)]
mod ffi {
    use super::*;

    extern "C" {
        /// 创建录像器
        ///
        /// # 参数
        /// * `codec_type` - 编解码器类型
        /// * `container_format` - 容器格式
        /// * `output_path` - 输出文件路径（C 字符串）
        ///
        /// # 返回
        /// * 成功：录像器句柄
        /// * 失败：NULL
        pub fn recorder_create(
            codec_type: i32,
            container_format: i32,
            output_path: *const c_char,
        ) -> RecorderHandle;

        /// 销毁录像器
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        pub fn recorder_destroy(handle: RecorderHandle);

        /// 开始录像
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        /// * `width` - 视频宽度（0 表示未知）
        /// * `height` - 视频高度（0 表示未知）
        /// * `framerate` - 帧率（0.0 表示未知）
        ///
        /// # 返回
        /// * 0：成功
        /// * 非 0：错误码
        pub fn recorder_start(
            handle: RecorderHandle,
            width: i32,
            height: i32,
            framerate: f64,
        ) -> i32;

        /// 写入视频帧
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        /// * `data` - 视频帧数据
        /// * `size` - 数据大小
        /// * `timestamp_ms` - 时间戳（毫秒，0 表示未知）
        /// * `keyframe` - 是否为关键帧（1：是，0：否）
        ///
        /// # 返回
        /// * 0：成功
        /// * 非 0：错误码
        pub fn recorder_write_frame(
            handle: RecorderHandle,
            data: *const u8,
            size: usize,
            timestamp_ms: i64,
            keyframe: i32,
        ) -> i32;

        /// 结束录像
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        ///
        /// # 返回
        /// * 0：成功
        /// * 非 0：错误码
        pub fn recorder_finish(handle: RecorderHandle) -> i32;

        /// 取消录像（不保存文件）
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        ///
        /// # 返回
        /// * 0：成功
        /// * 非 0：错误码
        pub fn recorder_cancel(handle: RecorderHandle) -> i32;

        /// 获取录像统计信息
        ///
        /// # 参数
        /// * `handle` - 录像器句柄
        /// * `stats` - 统计信息结构体指针
        ///
        /// # 返回
        /// * 0：成功
        /// * 非 0：错误码
        pub fn recorder_get_stats(
            handle: RecorderHandle,
            stats: *mut RecorderStatsFFI,
        ) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_recorder_new() {
        // 默认应该返回错误（未实现或 feature 未启用）
        let result = FfiRecorder::new(0, 0, "test.mp4");
        assert!(result.is_err());
    }

    #[test]
    fn test_recorder_stats_ffi_default() {
        let stats = RecorderStatsFFI::default();
        assert_eq!(stats.start_timestamp_ms, 0);
        assert_eq!(stats.end_timestamp_ms, 0);
        assert_eq!(stats.frames_written, 0);
        assert_eq!(stats.bytes_written, 0);
        assert_eq!(stats.duration_ms, 0);
    }
}
