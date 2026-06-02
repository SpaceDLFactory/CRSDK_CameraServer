// src/error.rs - 에러 처리

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrErrorCode(pub i32);

impl CrErrorCode {
    pub const NONE: Self = Self(0x0000);
    pub const GENERIC: Self = Self(0x8000u32 as i32);
    pub const FILE: Self = Self(0x8100u32 as i32);
    pub const CONNECT: Self = Self(0x8200u32 as i32);
    pub const MEMORY: Self = Self(0x8300u32 as i32);
    pub const API: Self = Self(0x8400u32 as i32);
    pub const INIT: Self = Self(0x8500u32 as i32);

    pub fn is_success(self) -> bool { self.0 == 0 }
    pub fn is_error(self) -> bool { self.0 != 0 }

    /// CrWarning(0x2xxxx) / CrWarningExt(0x6xxxx) 등 경고 코드 여부.
    /// CrError 카테고리는 0x8000–0x9xxx(< 0x20000)이므로, 0x20000 이상은 경고.
    /// 예: CrWarning_Frame_NotUpdated(0x20017) = LiveView 프레임 미갱신(에러 아님).
    pub fn is_warning(self) -> bool {
        (self.0 as u32) >= 0x0002_0000
    }
}

impl fmt::Display for CrErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CrErrorCode(0x{:08x})", self.0)
    }
}

pub enum SdkError {
    InitFailed,
    EnumFailed(CrErrorCode),
    IndexOutOfRange { index: u32, count: u32 },
    NullPointer,
    StringConversion,
    ConnectFailed(CrErrorCode),
    ConnectTimeout,
    LiveViewUnavailable,
    Sdk(CrErrorCode),
}

impl fmt::Debug for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdkError::InitFailed => write!(f, "InitFailed"),
            SdkError::EnumFailed(code) => write!(f, "EnumFailed({:?})", code),
            SdkError::IndexOutOfRange { index, count } => {
                write!(f, "IndexOutOfRange {{ index: {}, count: {} }}", index, count)
            }
            SdkError::NullPointer => write!(f, "NullPointer"),
            SdkError::StringConversion => write!(f, "StringConversion"),
            SdkError::ConnectFailed(code) => write!(f, "ConnectFailed({:?})", code),
            SdkError::ConnectTimeout => write!(f, "ConnectTimeout"),
            SdkError::LiveViewUnavailable => write!(f, "LiveViewUnavailable"),
            SdkError::Sdk(code) => write!(f, "Sdk({:?})", code),
        }
    }
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type SdkResult<T> = Result<T, SdkError>;