// src/shutter.rs — 셔터 릴리즈
//
// ── 전제조건 (호출 전 반드시 확인) ──────────────────────────────────────────
//
// capture() 는 PARAM_DOWN 이후 35ms 뒤 PARAM_UP을 보낸다.
// 이 35ms는 카메라가 MF(수동 초점) 상태일 때만 충분하다.
//
// AF 모드에서 capture()를 그냥 호출하면:
//   1. PARAM_DOWN → 카메라가 AF 탐색을 시작함
//   2. 35ms 후 PARAM_UP → AF가 아직 완료되지 않았으면 무시됨
//   3. 셔터가 눌리지 않거나 통신이 꼬임
//
// ── 올바른 사전 설정 (§11-2 참조) ───────────────────────────────────────────
//   1. SetDeviceProperty(PriorityKeySettings = PCRemote)
//   2. SetDeviceProperty(ExposureProgramMode = M_Manual)   ← 권장
//   3. SetDeviceProperty(FocusMode = MF)                   ← 필수 (AF 시 §참조)
//   4. SetDeviceProperty(StillImageStoreDestination = HostPC) ← PC 저장 시
//
// ── AF 모드에서 안전하게 촬영하려면 ─────────────────────────────────────────
// capture()를 직접 쓰지 말고, 이벤트 루프와 연동하는 방식을 사용해야 한다:
//
//   1. send(handle, CMD_RELEASE, PARAM_DOWN)  ← AF 탐색 시작
//   2. 이벤트 루프에서 대기:
//      - CrWarning_FocusPosition_Result_OK (0x8005) → PARAM_UP 발사 → 촬영
//      - CrWarning_FocusPosition_Result_NG (0x8006) → 포커스 실패 → PARAM_UP 후 에러
//      - timeout (예: 5초) → PARAM_UP 후 SdkError 반환
//
// capture()는 MF/사전-합초 전용이다. AF 대기 패턴은 Phase 5 이벤트 루프에서 구현.

use std::thread;
use std::time::Duration;

use crate::error::{CrErrorCode, SdkError, SdkResult};
use crate::ffi;

// CrCommandId (CrCommandData.h)
const CMD_RELEASE: u32 = 0x0000_0000; // CrCommandId_Release
const CMD_MOVIE_RECORD: u32 = 0x0000_0001; // CrCommandId_MovieRecord
const CMD_CANCEL_SHOOTING: u32 = 0x0000_0002; // CrCommandId_CancelShooting

// CrCommandParam (CrInt16u)
const PARAM_UP: u16   = 0x0000; // CrCommandParam_Up
const PARAM_DOWN: u16 = 0x0001; // CrCommandParam_Down

/// 단일 캡처: 셔터 Down → 35ms → 셔터 Up.
///
/// **전제조건**: 카메라가 MF(수동 초점) 모드여야 한다.
/// AF 모드에서 호출하면 포커스 완료 전에 PARAM_UP이 발사되어 촬영이 무시될 수 있다.
/// AF 연동 촬영은 이벤트 루프와 통합된 별도 로직 필요 (파일 상단 주석 참조).
pub fn capture(handle: i64) -> SdkResult<()> {
    send(handle, CMD_RELEASE, PARAM_DOWN)?;
    thread::sleep(Duration::from_millis(35));
    send(handle, CMD_RELEASE, PARAM_UP)
}

/// AF 모드 캡처: S1 반누름(AF lock) → 대기 → 셔터 Down/Up → S1 해제.
///
/// 공식 RemoteCli 샘플(`CameraDevice::af_shutter`)의 시간 기반 시퀀스를 따른다:
/// S1=Locked → 500ms(AF 합초 시간) → Release Down → 35ms → Up → S1=Unlocked.
/// FocusIndication 폴링 없이 시간으로 합초를 기다리므로 단순하다.
pub fn capture_af(handle: i64) -> SdkResult<()> {
    use crate::properties::{self, code, lock};
    // 1. S1 반누름 (AF 탐색 시작)
    properties::set(handle, code::S1, lock::LOCKED)?;
    thread::sleep(Duration::from_millis(500)); // AF 합초 대기
    // 2. 셔터
    send(handle, CMD_RELEASE, PARAM_DOWN)?;
    thread::sleep(Duration::from_millis(35));
    send(handle, CMD_RELEASE, PARAM_UP)?;
    thread::sleep(Duration::from_millis(300));
    // 3. S1 해제 (실패해도 캡처는 끝났으므로 best-effort)
    let _ = properties::set(handle, code::S1, lock::UNLOCKED);
    Ok(())
}

/// 셔터 누름 (Release Down). press-and-hold 연사용 — `shutter_up`까지 유지된다.
/// 누르고 있는 동안 드라이브 모드가 연속이면 카메라가 연속 촬영한다.
pub fn shutter_down(handle: i64) -> SdkResult<()> {
    send(handle, CMD_RELEASE, PARAM_DOWN)
}

/// 셔터 뗌 (Release Up).
pub fn shutter_up(handle: i64) -> SdkResult<()> {
    send(handle, CMD_RELEASE, PARAM_UP)
}

/// 동영상 녹화 시작 (MovieRecord Down). 정지는 `movie_record_stop`.
pub fn movie_record_start(handle: i64) -> SdkResult<()> {
    send(handle, CMD_MOVIE_RECORD, PARAM_DOWN)
}

/// 동영상 녹화 정지 (MovieRecord Up).
pub fn movie_record_stop(handle: i64) -> SdkResult<()> {
    send(handle, CMD_MOVIE_RECORD, PARAM_UP)
}

/// 촬영 취소 (CancelShooting). 벌브/연속/타이머 진행 중 중단.
pub fn cancel_shooting(handle: i64) -> SdkResult<()> {
    send(handle, CMD_CANCEL_SHOOTING, PARAM_DOWN)
}

fn send(handle: i64, cmd: u32, param: u16) -> SdkResult<()> {
    let err = unsafe { ffi::camera_send_command(handle, cmd, param) };
    let code = CrErrorCode(err);
    if code.is_error() { Err(SdkError::Sdk(code)) } else { Ok(()) }
}
