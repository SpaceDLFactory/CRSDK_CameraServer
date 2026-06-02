use std::ffi::c_void;

use crate::error::{CrErrorCode, SdkError, SdkResult};
use crate::ffi;

pub struct LiveViewStream {
    handle: i64,
    block_ptr: *mut c_void,
    buf_ptr: *mut u8,
}

unsafe impl Send for LiveViewStream {}

impl LiveViewStream {
    /// LiveView 스트림 초기화.
    /// 연결 직후 카메라가 LiveView를 준비하는 데 시간이 필요할 수 있으므로,
    /// `LiveViewUnavailable` 에러 시 잠시 대기 후 재시도하세요.
    pub fn new(handle: i64) -> SdkResult<Self> {
        let mut buf_size: u32 = 0;
        let err = unsafe { ffi::liveview_get_buffer_size(handle, &mut buf_size) };
        let code = CrErrorCode(err);
        if code.is_error() {
            return Err(SdkError::Sdk(code));
        }
        if buf_size == 0 {
            return Err(SdkError::LiveViewUnavailable);
        }

        let mut buf_ptr: *mut u8 = std::ptr::null_mut();
        let block_ptr = unsafe { ffi::liveview_alloc_block(buf_size, &mut buf_ptr) };
        if block_ptr.is_null() || buf_ptr.is_null() {
            return Err(SdkError::NullPointer);
        }

        Ok(Self { handle, block_ptr, buf_ptr })
    }

    /// 현재 LiveView 프레임(JPEG)을 `Vec<u8>`으로 반환.
    /// 새 프레임이 아직 준비되지 않으면 빈 Vec.
    pub fn fetch_frame(&self) -> SdkResult<Vec<u8>> {
        let mut image_size: u32 = 0;
        let mut image_data: *const u8 = std::ptr::null();

        let err = unsafe {
            ffi::liveview_fetch(
                self.handle,
                self.block_ptr,
                &mut image_size,
                &mut image_data,
            )
        };

        let code = CrErrorCode(err);
        // CrWarning_Frame_NotUpdated(0x20017) 등 경고는 "새 프레임 없음"으로 취급.
        // 경고를 하드 에러로 반환하면 스트림이 첫 프레임 후 끊긴다.
        if code.is_error() && !code.is_warning() {
            return Err(SdkError::Sdk(code));
        }
        if image_data.is_null() || image_size == 0 {
            return Ok(Vec::new());
        }

        let data = unsafe { std::slice::from_raw_parts(image_data, image_size as usize) };
        Ok(data.to_vec())
    }
}

impl Drop for LiveViewStream {
    fn drop(&mut self) {
        unsafe { ffi::liveview_free_block(self.block_ptr, self.buf_ptr) };
    }
}

/// 카메라 중력센서(자이로) 레벨 정보. `on=false`면 카메라가 레벨을 안 줌(roll 무의미).
#[derive(Debug, Clone, Copy)]
pub struct LevelInfo {
    pub on: bool,
    pub roll: i32,  // x
    pub pitch: i32, // y
    pub z: i32,
}

/// LiveView Level(자이로) 1회 조회. `CrLiveViewProperty_Level`을 읽는다.
pub fn get_level(handle: i64) -> SdkResult<LevelInfo> {
    let mut out: ffi::CrLevelSimple = unsafe { std::mem::zeroed() };
    let err = unsafe { ffi::liveview_get_level(handle, &mut out) };
    let code = CrErrorCode(err);
    if code.is_error() && !code.is_warning() {
        return Err(SdkError::Sdk(code));
    }
    Ok(LevelInfo { on: out.state == 2, roll: out.x, pitch: out.y, z: out.z })
}

/// AF 프레임 실위치. 위치는 분수 (x=x_num/x_deno 등). `valid=false`면 프레임 없음.
#[derive(Debug, Clone, Copy)]
pub struct AfFrame {
    pub valid: bool,
    pub x_num: u32, pub x_deno: u32,
    pub y_num: u32, pub y_deno: u32,
    pub width: u32, pub height: u32,
}

/// LiveView AF_Area_Position(0x0121)을 읽어 카메라가 실제 놓은 박스 위치를 반환.
pub fn get_af_frame(handle: i64) -> SdkResult<AfFrame> {
    let mut out: ffi::CrAfFrameSimple = unsafe { std::mem::zeroed() };
    let err = unsafe { ffi::liveview_get_af_frame(handle, &mut out) };
    let code = CrErrorCode(err);
    if code.is_error() && !code.is_warning() {
        return Err(SdkError::Sdk(code));
    }
    Ok(AfFrame {
        valid: out.valid != 0,
        x_num: out.x_num, x_deno: out.x_deno,
        y_num: out.y_num, y_deno: out.y_deno,
        width: out.width, height: out.height,
    })
}
