use std::ffi::{c_void, CStr};
use std::os::raw::c_char;

use crate::error::{CrErrorCode, SdkError, SdkResult};
use crate::ffi;
use crate::session::SdkSession;

/// 개별 카메라 정보 (ICrCameraObjectInfo에서 추출).
#[derive(Debug, Clone)]
pub struct CameraInfo {
    /// 디바이스 이름 (GetName)
    pub name: String,
    /// 모델 이름 (GetModel)
    pub model: String,
    /// USB Product ID
    pub usb_pid: u16,
    /// 현재 연결 상태 코드 (GetConnectionStatus)
    pub connection_status: u32,
    /// true = WiFi/SSH 카메라, false = USB PTP 카메라
    pub ssh_support: bool,
}

/// 카메라 열거 결과 핸들.
/// Drop 시 자동으로 enum_release()를 호출하여 SDK 메모리 해제.
pub struct CameraEnumerator {
    /// ICrEnumCameraObjectInfo* (opaque)
    handle: *mut c_void,
    count: u32,
}

// Safety: SDK 핸들은 생성한 스레드에서만 사용. Send만 허용.
unsafe impl Send for CameraEnumerator {}

impl CameraEnumerator {
    /// 연결된 카메라를 최대 `timeout_sec`초 동안 탐색.
    ///
    /// `_session`은 SDK가 초기화된 상태임을 컴파일 타임에 보장하기 위한 참조.
    pub fn new(_session: &SdkSession, timeout_sec: u8) -> SdkResult<Self> {
        let mut handle: *mut c_void = std::ptr::null_mut();

        let err = unsafe {
            ffi::enum_cameras(
                &mut handle as *mut *mut c_void,
                timeout_sec,
            )
        };

        let code = CrErrorCode(err);
        if code.is_error() {
            return Err(SdkError::EnumFailed(code));
        }

        let count = if handle.is_null() {
            0
        } else {
            unsafe { ffi::enum_get_count(handle as *const c_void) }
        };

        Ok(Self { handle, count })
    }

    /// 발견된 카메라 수.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// `index`번째 카메라 정보를 반환.
    pub fn get(&self, index: u32) -> SdkResult<CameraInfo> {
        if index >= self.count {
            return Err(SdkError::IndexOutOfRange { index, count: self.count });
        }

        let cam = unsafe {
            ffi::enum_get_camera_ptr(self.handle as *const c_void, index)
        };
        if cam.is_null() {
            return Err(SdkError::NullPointer);
        }

        let name = read_cchar(unsafe { ffi::camera_get_name_ptr(cam) })?;
        let model = read_cchar(unsafe { ffi::camera_get_model_ptr(cam) })?;
        let usb_pid = unsafe { ffi::camera_get_usb_pid(cam) };
        let connection_status = unsafe { ffi::camera_get_connection_status(cam) };
        let ssh_support = unsafe { ffi::camera_get_ssh_support(cam) } != 0;

        Ok(CameraInfo { name, model, usb_pid, connection_status, ssh_support })
    }

    /// 모든 카메라를 Vec로 수집.
    pub fn list_all(&self) -> SdkResult<Vec<CameraInfo>> {
        (0..self.count).map(|i| self.get(i)).collect()
    }

    /// WiFi(SSH) 카메라의 SSH 호스트 키 핑거프린트를 가져온다.
    /// 네트워크 왕복이 발생하므로 블로킹. USB 카메라에서는 빈 Vec 반환.
    pub fn get_fingerprint(&self, index: u32) -> SdkResult<Vec<u8>> {
        let cam = self.camera_ptr(index)?;
        let mut buf = vec![0u8; 1024];
        let len = unsafe {
            ffi::camera_get_fingerprint(
                cam,
                buf.as_mut_ptr() as *mut std::os::raw::c_char,
                buf.len() as u32,
            )
        };
        buf.truncate(len as usize);
        if buf.last() == Some(&0) { buf.pop(); }
        Ok(buf)
    }

    /// camera_connect() 호출용 opaque ICrCameraObjectInfo* 반환.
    /// 반환된 포인터는 이 `CameraEnumerator`가 살아있는 동안만 유효.
    pub fn camera_ptr(&self, index: u32) -> SdkResult<*const c_void> {
        if index >= self.count {
            return Err(SdkError::IndexOutOfRange { index, count: self.count });
        }
        let ptr = unsafe {
            ffi::enum_get_camera_ptr(self.handle as *const c_void, index)
        };
        if ptr.is_null() {
            Err(SdkError::NullPointer)
        } else {
            Ok(ptr)
        }
    }
}

impl Drop for CameraEnumerator {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { ffi::enum_release(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

/// `*const c_char` (null-terminated, UTF-8) → `String`.
/// CrChar = char on macOS (no UNICODE define).
fn read_cchar(ptr: *const c_char) -> SdkResult<String> {
    if ptr.is_null() {
        return Ok(String::new());
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_str()
            .map(|s| s.to_owned())
            .map_err(|_| SdkError::StringConversion)
    }
}
