// src/control.rs — ControlCode 기반 즉시 작용 명령
//
// SDK::ExecuteControlCodeValue 의 안전 래퍼.
// NearFar(MF 초점 이동), AFAreaPosition, ZoomOperation 등 즉시 동작 명령에 사용.

use crate::error::{CrErrorCode, SdkError, SdkResult};
use crate::ffi;

/// CrControlCode 값 (CrControlCode.h)
pub mod code {
    pub const NEAR_FAR: u32 = 0x0000_D2D1;          // MF 포커스 이동 (부호=방향, 크기=스텝)
    pub const AF_AREA_POSITION: u32 = 0x0000_D2DC;  // AF 영역 좌표 이동
    pub const ZOOM_OPERATION: u32 = 0x0000_D2DD;    // 줌 동작
}

/// AF 영역 위치 지정. x,y 는 0~10000 정규화 좌표 (좌상단 0,0).
/// SDK는 ExecuteControlCodeValue에 (x<<16)|y 형태로 패킹된 좌표를 받는다.
/// 실제 스케일은 모델/렌즈에 따라 다를 수 있어, 카메라가 거부하면
/// get_info(AF_AREA_POSITION)로 유효 범위를 확인해 보정해야 한다.
pub fn af_area_position(handle: i64, x: u16, y: u16) -> SdkResult<()> {
    let packed = ((x as u64) << 16) | (y as u64);
    execute(handle, code::AF_AREA_POSITION, packed)
}

/// CrDataType 비트 (CrDefines.h):
///   base    : 0x000F  (1=u8 2=u16 3=u32 4=u64 5=u128)
///   SignBit : 0x1000  (부호)
///   ArrayBit: 0x2000  (값 목록 enum)
///   RangeBit: 0x4000  (`[min, step, max]` 3원소)
pub mod data_type {
    pub const BASE_MASK: u32 = 0x000F;
    pub const SIGN_BIT: u32 = 0x1000;
    pub const ARRAY_BIT: u32 = 0x2000;
    pub const RANGE_BIT: u32 = 0x4000;
}

/// ControlCode 의 허용 값/범위 정보.
#[derive(Debug, Clone)]
pub struct ControlInfo {
    pub value_type: u32,
    /// raw 원소(부호 해석은 caller가 value_type 보고 결정).
    /// RangeBit이 켜져 있으면 values[0..3] = [min, max, step] (실측 — 후행 0 padding 가능).
    /// ArrayBit이면 enum 목록.
    pub values: Vec<u64>,
}

impl ControlInfo {
    pub fn is_range(&self) -> bool {
        self.value_type & data_type::RANGE_BIT != 0
    }
    pub fn is_array(&self) -> bool {
        self.value_type & data_type::ARRAY_BIT != 0
    }
    pub fn is_signed(&self) -> bool {
        self.value_type & data_type::SIGN_BIT != 0
    }
}

/// GetSelectControlCode 안전 래퍼.
pub fn get_info(handle: i64, code: u32) -> SdkResult<ControlInfo> {
    let mut out: ffi::CrControlInfoSimple = unsafe { std::mem::zeroed() };
    let err = unsafe { ffi::get_control_code_info(handle, code, &mut out) };
    let c = CrErrorCode(err);
    if c.is_error() && !c.is_warning() {
        return Err(SdkError::Sdk(c));
    }
    let n = (out.count as usize).min(out.values.len());
    Ok(ControlInfo {
        value_type: out.value_type,
        values: out.values[..n].to_vec(),
    })
}

/// ControlCode 실행.
pub fn execute(handle: i64, code: u32, value: u64) -> SdkResult<()> {
    let err = unsafe { ffi::execute_control_code_value(handle, code, value) };
    let c = CrErrorCode(err);
    if c.is_error() && !c.is_warning() {
        Err(SdkError::Sdk(c))
    } else {
        Ok(())
    }
}

/// MF 초점 이동. `step` 부호=방향(음수=Near/가까이, 양수=Far/멀리), 크기=스텝.
/// Sony 일반적 범위 ±1(미세) ~ ±7(거침). 모델/렌즈에 따라 유효값 다름.
/// 부호 확장으로 u64에 패킹 (SDK가 i32/i64 어느 쪽으로 해석해도 일관).
pub fn focus_near_far(handle: i64, step: i32) -> SdkResult<()> {
    execute(handle, code::NEAR_FAR, step as i64 as u64)
}
