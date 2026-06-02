use crate::error::{SdkError, SdkResult};
use crate::ffi;

/// RAII 패턴으로 Sony SDK 수명 관리.
/// Drop 시 자동으로 sdk_release() 호출.
///
/// # 주의
/// 프로세스당 하나의 SdkSession만 존재해야 함.
/// SDK 자체가 다중 초기화를 지원하지 않음.
pub struct SdkSession {
    _private: (),
}

impl SdkSession {
    /// SDK 초기화. log_type: 0 = 로그 없음.
    pub fn new(log_type: i32) -> SdkResult<Self> {
        let ret = unsafe { ffi::sdk_init(log_type) };
        if ret == 0 {
            Ok(Self { _private: () })
        } else {
            Err(SdkError::InitFailed)
        }
    }

    /// SDK 버전 (BCD 형식: 0x02010000 = v2.01.00).
    pub fn version(&self) -> u32 {
        unsafe { ffi::get_sdk_version() }
    }
}

impl Drop for SdkSession {
    fn drop(&mut self) {
        unsafe {
            ffi::sdk_release();
        }
    }
}
