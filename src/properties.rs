// src/properties.rs — 디바이스 속성 읽기/쓰기
//
// get_device_properties / set_device_property (C 래퍼)를 안전 래핑한다.
// set은 wrapper.cpp의 Fetch-Modify-Set 패턴을 사용하므로 code + current_value만
// 채운 CrPropertySimple을 넘기면 된다 (나머지는 0).

use crate::error::{CrErrorCode, SdkError, SdkResult};
use crate::ffi;

/// CrDevicePropertyCode (CrDeviceProperty.h enum을 정확히 파싱한 값).
/// enum이 순차 증가가 아니라 중간 항목(ExposureBias, FlashComp, FileType 등)이
/// 끼어 있으므로 가정하지 말고 헤더의 실제 값과 일치시킬 것.
pub mod code {
    pub const S1: u32 = 0x0001; // 셔터 반누름 (AF lock 트리거)
    pub const F_NUMBER: u32 = 0x0100;
    pub const EXPOSURE_BIAS_COMPENSATION: u32 = 0x0101;
    pub const SHUTTER_SPEED: u32 = 0x0103;
    pub const ISO_SENSITIVITY: u32 = 0x0104;
    pub const EXPOSURE_PROGRAM_MODE: u32 = 0x0105;
    pub const FILE_TYPE: u32 = 0x0106;
    pub const WHITE_BALANCE: u32 = 0x0108;
    pub const METERING_MODE: u32 = 0x010A;
    pub const DRIVE_MODE: u32 = 0x010E;
    pub const FOCUS_MODE: u32 = 0x0109;
    pub const LENS_MODEL_NAME: u32 = 0x0765; // STR 타입 (A7C 미노출 — 다른 바디용)
    pub const RECORDING_STATE: u32 = 0x0705; // 0=정지 1=녹화중 2=실패 (읽기 전용)
    pub const SHUTTER_TYPE: u32 = 0x01A9;    // 1=Auto 2=기계 3=전자
    pub const SILENT_MODE: u32 = 0x01A5;     // 1=Off 2=On
    pub const BATTERY_REMAIN: u32 = 0x0702;  // 잔량 % (0xFFFF=미취득), 읽기 전용
    pub const MEDIA_SLOT1_REMAINING_NUMBER: u32 = 0x0709; // 남은 컷, 읽기 전용
    pub const FOCUS_AREA: u32 = 0x0113;
    pub const STILL_IMAGE_STORE_DESTINATION: u32 = 0x0119;
    pub const PRIORITY_KEY_SETTINGS: u32 = 0x011A;
    pub const STILL_IMAGE_QUALITY: u32 = 0x0107;        // CrImageQuality (Light~ExFine)
    pub const PICTURE_PROFILE: u32 = 0x01AA;            // Off/PP1~PP11 (A7C 노출)
    pub const COLOR_TEMP: u32 = 0x0115;                // 켈빈값 (WB=색온도 모드일 때만 editable)
    pub const AF_AREA_POSITION: u32 = 0x0121;          // UInt32 (x<<16)|y, x:0~639 y:0~479. FocusArea=FlexibleSpot 필요
    pub const FOCUS_INDICATION: u32 = 0x0707;          // 합초 상태 (읽기 전용)
}

/// CrFocusArea 값 (Flexible Spot 크기)
pub mod focus_area {
    pub const FLEXIBLE_SPOT_S: u64 = 0x0004;
    pub const FLEXIBLE_SPOT_M: u64 = 0x0005;
    pub const FLEXIBLE_SPOT_L: u64 = 0x0006;
}

/// CrFileType 값 (RAW/JPEG 선택)
pub mod file_type {
    pub const JPEG: u64 = 0x0001;
    pub const RAW: u64 = 0x0002;
    pub const RAW_JPEG: u64 = 0x0003;
    pub const RAW_HEIF: u64 = 0x0004;
    pub const HEIF: u64 = 0x0005;
}

/// CrFocusMode 값
pub mod focus_mode {
    pub const MF: u64 = 0x0001;
    pub const AF_S: u64 = 0x0002;
    pub const AF_C: u64 = 0x0003;
    pub const AF_A: u64 = 0x0004;
    pub const AF_D: u64 = 0x0005;
    pub const DMF: u64 = 0x0006;
    pub const PF: u64 = 0x0007;
}

/// CrStillImageStoreDestination 값
pub mod save_dest {
    pub const HOST_PC: u64 = 0x0001;
    pub const MEMORY_CARD: u64 = 0x0002;
    pub const HOST_PC_AND_MEMORY_CARD: u64 = 0x0003;
}

/// CrExposureProgram 값 (일부)
pub mod exposure_program {
    pub const M_MANUAL: u64 = 0x0001;
}

/// CrPriorityKeySettings 값
pub mod priority_key {
    pub const CAMERA_POSITION: u64 = 0x0001;
    pub const PC_REMOTE: u64 = 0x0002;
}

/// CrLockIndicator 값 (S1 반누름 제어)
pub mod lock {
    pub const UNLOCKED: u64 = 0x0001;
    pub const LOCKED: u64 = 0x0002;
}

#[derive(Debug, Clone)]
pub struct Property {
    pub code: u32,
    pub value_type: u32,
    pub current: u64,
    pub editable: bool,
    pub allowed: Vec<u64>,
}

/// 디바이스의 모든 속성을 조회.
pub fn get_all(handle: i64) -> SdkResult<Vec<Property>> {
    let mut ptr: *mut ffi::CrPropertySimple = std::ptr::null_mut();
    let mut count: u32 = 0;
    let err = unsafe { ffi::get_device_properties(handle, &mut ptr, &mut count) };
    let code = CrErrorCode(err);
    if code.is_error() && !code.is_warning() {
        return Err(SdkError::Sdk(code));
    }
    if ptr.is_null() || count == 0 {
        return Ok(Vec::new());
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, count as usize) };
    let props = slice
        .iter()
        .map(|p| {
            let n = (p.allowed_count as usize).min(p.allowed_values.len());
            Property {
                code: p.code,
                value_type: p.value_type,
                current: p.current_value,
                editable: p.is_editable != 0,
                allowed: p.allowed_values[..n].to_vec(),
            }
        })
        .collect();

    unsafe { ffi::release_device_properties_simple(ptr) };
    Ok(props)
}

/// 단일 속성 조회 (code 일치하는 첫 항목).
pub fn get(handle: i64, code: u32) -> SdkResult<Option<Property>> {
    Ok(get_all(handle)?.into_iter().find(|p| p.code == code))
}

/// STR 타입 속성의 현재 문자열 조회 (예: LensModelName).
/// `None` = 속성 없음/문자열 비어 있음.
pub fn get_string(handle: i64, code: u32) -> SdkResult<Option<String>> {
    let mut buf = [0u8; 256];
    let n = unsafe {
        ffi::get_property_string(handle, code, buf.as_mut_ptr() as *mut _, buf.len() as u32)
    };
    if n <= 0 {
        return Ok(None);
    }
    let s = std::str::from_utf8(&buf[..n as usize])
        .map_err(|_| SdkError::StringConversion)?
        .to_string();
    Ok(if s.is_empty() { None } else { Some(s) })
}

/// 속성 쓰기. wrapper.cpp가 Fetch-Modify-Set으로 처리하므로
/// code + current_value만 채운 구조체를 넘긴다.
pub fn set(handle: i64, code: u32, value: u64) -> SdkResult<()> {
    let mut prop: ffi::CrPropertySimple = unsafe { std::mem::zeroed() };
    prop.code = code;
    prop.current_value = value;

    let err = unsafe { ffi::set_device_property(handle, &prop) };
    let c = CrErrorCode(err);
    if c.is_error() && !c.is_warning() {
        Err(SdkError::Sdk(c))
    } else {
        Ok(())
    }
}
