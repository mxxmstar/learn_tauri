//! DMON（设备监控）操作
//!
//! 通过 FFI 调用底层 C 的 DMON 函数，实现设备的重启、版本查询、
//! Ping、同步、内存读写、深度睡眠等功能。

use super::error::{from_bcm_err, BcmError};
use super::ffi;
use super::types::*;

/// 重启设备
pub fn reboot(hdl: u64) -> Result<(), BcmError> {
    let ret = unsafe { ffi::DMON_Reboot(hdl) };
    if ret != BCM_ERR_OK {
        Err(from_bcm_err(ret))
    } else {
        Ok(())
    }
}

/// 获取软件版本字符串
pub fn get_sw_version(hdl: u64) -> Result<String, BcmError> {
    let mut ver = DMON_SwVersionMsgType { str: [0i8; 100] };

    let ret = unsafe { ffi::DMON_GetSwVersion(hdl, &mut ver) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    // 将 C 字符串（以 0 结尾）转为 Rust String
    let bytes: Vec<u8> = ver
        .str
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as u8)
        .collect();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// 获取硬件版本信息
pub fn get_hw_version(hdl: u64) -> Result<DMON_HwVersionMsgType, BcmError> {
    let mut ver = DMON_HwVersionMsgType {
        manuf: 0,
        model: 0,
        rev: 0,
        sec_mode: 0,
    };

    let ret = unsafe { ffi::DMON_GetHwVersion(hdl, &mut ver) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(ver)
}

/// Ping 设备，获取启动模式和硬件版本信息
pub fn ping(hdl: u64) -> Result<DMON_PingMsgType, BcmError> {
    let mut ping = DMON_PingMsgType {
        mode: 0,
        version: DMON_HwVersionMsgType {
            manuf: 0,
            model: 0,
            rev: 0,
            sec_mode: 0,
        },
    };

    let ret = unsafe { ffi::DMON_Ping(hdl, &mut ping) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(ping)
}

/// 获取设备同步状态（含启动模式、设备状态、各阶段时间等）
pub fn sync(hdl: u64) -> Result<DMON_SyncMsgType, BcmError> {
    let mut sync = DMON_SyncMsgType {
        mode: 0,
        state: 0,
        version: DMON_HwVersionMsgType {
            manuf: 0,
            model: 0,
            rev: 0,
            sec_mode: 0,
        },
        init_time: 0,
        ready_time: 0,
        run_time: 0,
        rsvd: [0u8; 208],
    };

    let ret = unsafe { ffi::DMON_Sync(hdl, &mut sync) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(sync)
}

/// 等待设备达到指定状态后返回同步信息
pub fn sync_wait(hdl: u64, state: u32) -> Result<DMON_SyncMsgType, BcmError> {
    let mut sync = DMON_SyncMsgType {
        mode: 0,
        state: 0,
        version: DMON_HwVersionMsgType {
            manuf: 0,
            model: 0,
            rev: 0,
            sec_mode: 0,
        },
        init_time: 0,
        ready_time: 0,
        run_time: 0,
        rsvd: [0u8; 208],
    };

    let ret = unsafe { ffi::DMON_SyncWait(hdl, state, &mut sync) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(sync)
}

/// 读取设备内存
pub fn read_mem(hdl: u64, addr: u32, width: u32, device_id: u32) -> Result<u32, BcmError> {
    let mut data: u32 = 0;

    let ret = unsafe { ffi::DMON_ReadMem(hdl, addr, width, device_id, &mut data) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(data)
}

/// 写入设备内存
pub fn write_mem(
    hdl: u64,
    addr: u32,
    width: u32,
    device_id: u32,
    data: u32,
) -> Result<(), BcmError> {
    let ret = unsafe { ffi::DMON_WriteMem(hdl, addr, width, device_id, data) };
    if ret != BCM_ERR_OK {
        return Err(from_bcm_err(ret));
    }

    Ok(())
}

/// 让设备进入深度睡眠模式
pub fn deep_sleep(hdl: u64) -> Result<(), BcmError> {
    let ret = unsafe { ffi::DMON_DeepSleep(hdl) };
    if ret != BCM_ERR_OK {
        Err(from_bcm_err(ret))
    } else {
        Ok(())
    }
}
