//! CONFIG（设备配置）读写操作
//!
//! 通过 FFI 调用 C 层的 CONFIG_RpcRead / CONFIG_RpcWrite 实现配置的读取和写入。
//! 同时提供 write_config_msg 函数，将人类可读的配置键值对序列化为设备可识别的二进制格式。

use std::ffi::CStr;
use std::os::raw::c_char;

use super::error::{from_bcm_err, BcmError};
use super::ffi;
use super::types::*;

/// 配置项名称缓冲区大小
const CONFIG_ITEM_NAME_MAX: usize = 32;
/// 配置项值缓冲区大小
const CONFIG_ITEM_VAL_MAX: usize = 32;

/// 从设备读取原始配置数据
pub fn read_raw(hdl: u64) -> Result<CONFIG_RpcMsg, BcmError> {
    let mut msg = CONFIG_RpcMsg {
        ctx: [0u8; 256],
        len: 256,
    };

    let ret = unsafe { ffi::CONFIG_RpcRead(hdl, &mut msg) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(msg)
}

/// 将原始配置数据写入设备
pub fn write_raw(hdl: u64, msg: &mut CONFIG_RpcMsg) -> Result<(), BcmError> {
    let ret = unsafe { ffi::CONFIG_RpcWrite(hdl, msg) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }
    Ok(())
}

/// 从配置缓冲区中解析单个配置项
///
/// # 参数
/// * `ctx` - 配置数据缓冲区
/// * `offset` - 当前解析偏移量
///
/// # 返回值
/// * `Ok((name, value, len))` - 配置项名称、值、占用字节数
/// * `Err(BcmError)` - 解析失败
pub fn extract_item(ctx: &[u8], offset: u32) -> Result<(String, String, u32), BcmError> {
    if offset as usize >= ctx.len() {
        return Err(BcmError::InvalidParams);
    }

    let mut name = vec![0i8; CONFIG_ITEM_NAME_MAX];
    let mut val = vec![0i8; CONFIG_ITEM_VAL_MAX];

    let len = unsafe {
        ffi::CONFIG_ExtractItem(
            ctx.as_ptr() as *mut u8,
            name.as_mut_ptr() as *mut c_char,
            val.as_mut_ptr() as *mut c_char,
        )
    };

    if len <= 4 {
        return Err(BcmError::InvalidParams);
    }

    let name = unsafe { CStr::from_ptr(name.as_ptr()).to_string_lossy().to_string() };
    let value = unsafe { CStr::from_ptr(val.as_ptr()).to_string_lossy().to_string() };

    Ok((name, value, len))
}

/// 解析全部配置项，返回键值对列表
pub fn parse_all(msg: &CONFIG_RpcMsg) -> Vec<ConfigPair> {
    let mut pairs = Vec::new();
    let mut offset = 0u32;

    while (offset as usize) < msg.ctx.len() {
        match extract_item(&msg.ctx, offset) {
            Ok((name, value, len)) => {
                pairs.push(ConfigPair { name, value });
                offset += len;
            }
            Err(_) => break,
        }
    }

    pairs
}

// ====== 配置项序列化（写配置） ======

/// 构造配置项头部（对应 C 的 CONFIG_ITEM_HEADER_R 宏）
///
/// 编码格式：
/// - bits[7:0]   = 0xAB（固定标志）
/// - bits[15:8]  = id 高字节
/// - bits[23:16] = id 低字节
/// - bits[31:24] = 数据长度
fn config_item_header(id: u16, len: u8) -> u32 {
    0xABu32 | ((id as u32 >> 8) & 0xFF) << 8 | (id as u32 & 0xFF) << 16 | (len as u32) << 24
}

/// 以小端序写入 4 字节整数到缓冲区
fn put_u32_le(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset] = (val & 0xFF) as u8;
    buf[offset + 1] = ((val >> 8) & 0xFF) as u8;
    buf[offset + 2] = ((val >> 16) & 0xFF) as u8;
    buf[offset + 3] = ((val >> 24) & 0xFF) as u8;
}

/// 将人类可读的配置名值对序列化为设备可识别的二进制消息
///
/// # 参数
/// * `name` - 配置项名称（如 "mirror mode", "FPS", "IP" 等）
/// * `val` - 配置项值
///
/// # 返回值
/// * `Ok(Vec<u8>)` - 序列化后的二进制数据
/// * `Err(BcmError)` - 不支持的配置项名称
pub fn write_config_msg(name: &str, val: &str) -> Result<Vec<u8>, BcmError> {
    let mut msg = [0u8; 256];
    let mut index = 0usize;

    match name {
        "mirror mode" => {
            let header = config_item_header(CONFIG_MEDIA_MIRROR, 1);
            put_u32_le(&mut msg, index, header);
            index += 4;
            msg[index] = val.parse::<u8>().unwrap_or(0);
        }
        "FPS" => {
            let header = config_item_header(CONFIG_MEDIA_FPS, 1);
            put_u32_le(&mut msg, index, header);
            index += 4;
            msg[index] = val.parse::<u8>().unwrap_or(0);
        }
        "SOMEIP UDP port" => {
            let header = config_item_header(CONFIG_MEDIA_SOMEIPUDPPORT, 2);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let v = val.parse::<u16>().unwrap_or(0);
            msg[index] = (v >> 8) as u8;
            msg[index + 1] = (v & 0xFF) as u8;
            index += 2;
        }
        "SOMEIP RTP port" => {
            let header = config_item_header(CONFIG_MEDIA_SOMEIPRTPPORT, 2);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let v = val.parse::<u16>().unwrap_or(0);
            msg[index] = (v >> 8) as u8;
            msg[index + 1] = (v & 0xFF) as u8;
            index += 2;
        }
        "DHCP" => {
            let header = config_item_header(CONFIG_NETWORK_DHCP, 1);
            put_u32_le(&mut msg, index, header);
            index += 4;
            msg[index] = val.parse::<u8>().unwrap_or(0);
        }
        "IP" => {
            let header = config_item_header(CONFIG_NETWORK_IP, 16);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let bytes = val.as_bytes();
            let len = bytes.len().min(16);
            msg[index..index + len].copy_from_slice(&bytes[..len]);
            index += 16;
        }
        "MAC" => {
            let header = config_item_header(CONFIG_NETWORK_MAC, 20);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let bytes = val.as_bytes();
            let len = bytes.len().min(20);
            msg[index..index + len].copy_from_slice(&bytes[..len]);
            index += 20;
        }
        "AVTP DST MAC" => {
            let header = config_item_header(CONFIG_AVTP_DSTMAC, 20);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let bytes = val.as_bytes();
            let len = bytes.len().min(20);
            msg[index..index + len].copy_from_slice(&bytes[..len]);
            index += 20;
        }
        "AVTP stream ID" => {
            let header = config_item_header(CONFIG_AVTP_STREAMID, 8);
            put_u32_le(&mut msg, index, header);
            index += 4;
            let v = u64::from_str_radix(val, 16).unwrap_or(0);
            // 大端序写入 8 字节
            msg[index] = ((v >> 56) & 0xFF) as u8;
            msg[index + 1] = ((v >> 48) & 0xFF) as u8;
            msg[index + 2] = ((v >> 40) & 0xFF) as u8;
            msg[index + 3] = ((v >> 32) & 0xFF) as u8;
            msg[index + 4] = ((v >> 24) & 0xFF) as u8;
            msg[index + 5] = ((v >> 16) & 0xFF) as u8;
            msg[index + 6] = ((v >> 8) & 0xFF) as u8;
            msg[index + 7] = (v & 0xFF) as u8;
            index += 8;
        }
        _ => return Err(BcmError::InvalidParams),
    }

    let mut result = Vec::with_capacity(index);
    result.extend_from_slice(&msg[..index]);
    Ok(result)
}
