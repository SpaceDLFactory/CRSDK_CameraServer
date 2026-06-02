use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crsdk::connection::{Camera, ConnectMode};
use crsdk::enumerate::CameraEnumerator;
use crsdk::error::SdkError;
use crsdk::session::SdkSession;

// ── macOS: USB 간섭 프로세스 RAII 억제 ─────────────────────────────────────
//
// Sony SDK가 USB PTP 어댑터를 생성(CrError_Adaptor_Create)하려 할 때
// 아래 두 프로세스가 USB 인터페이스를 선점하면 즉시 실패한다:
//
//   1. ptpcamerad   — macOS 내장 PTP 데몬 (launchd가 on-demand로 재시작)
//   2. "Android File Transfer Agent" — 설치된 경우 Sony PTP 카메라도 선점
//
// 대책:
//   • 시작 시 one-shot kill + launchctl stop (ptpcamerad 재시작 억제)
//   • 백그라운드 kill loop: 50ms 간격으로 재시작 시도를 차단 (CPU ~0%)
//   • Drop이 kill + wait을 보장 → 좀비 프로세스 없음
//
// 주의: std::process::exit()는 Drop을 건너뜀.
//   → main()에서 _killer를 명시적으로 drop한 뒤 exit() 호출할 것.

struct UsbInterferenceSuppressor(std::process::Child);

impl UsbInterferenceSuppressor {
    fn start() -> Option<Self> {
        // one-shot: 현재 실행 중인 인스턴스 즉시 제거
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "ptpcamerad"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "Android File Transfer Agent"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        // launchd가 ptpcamerad를 즉시 재시작하지 않도록 stop 요청
        let _ = std::process::Command::new("launchctl")
            .args(["stop", "com.apple.ptpcamerad"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        // 백그라운드 kill loop: 50ms 주기로 재시작 시도를 차단
        // sleep 추가로 CPU 사용률 ~0% 유지
        let child = std::process::Command::new("bash")
            .args([
                "-c",
                "while :; do \
                    pkill -KILL ptpcamerad 2>/dev/null; \
                    pkill -KILL 'Android File Transfer Agent' 2>/dev/null; \
                    sleep 0.05; \
                done",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;
        Some(Self(child))
    }
}

// 이전 이름 유지 (Drop 보장 패턴 동일)
type PtpDaemonKiller = UsbInterferenceSuppressor;

impl Drop for UsbInterferenceSuppressor {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait(); // 좀비 프로세스 방지
    }
}

// ── 에러 표시 ────────────────────────────────────────────────────────────────

struct DisplayError(SdkError);
impl fmt::Display for DisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

// ── 진입점 ───────────────────────────────────────────────────────────────────

fn main() {
    // ── Ctrl+C → graceful shutdown ──────────────────────────────────────────
    // Arc<AtomicBool> is shared between the signal handler and the main loop.
    // When the handler sets it to true the loop exits normally, allowing all
    // RAII Drop handlers (Camera, SdkSession, PtpDaemonKiller) to run.
    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let flag = Arc::clone(&shutdown);
        ctrlc::set_handler(move || {
            flag.store(true, Ordering::Relaxed);
        })
        .expect("failed to install Ctrl+C handler");
    }

    // USB 간섭 억제 시작 (ptpcamerad + Android File Transfer Agent)
    let killer = PtpDaemonKiller::start();

    // kill loop + launchctl stop이 효과를 발휘할 시간 확보
    // 500ms: ptpcamerad 재시작 주기(~100ms)보다 충분히 길게
    std::thread::sleep(Duration::from_millis(500));

    println!("── Sony Camera Remote SDK Phase 3 Test ──");

    let result = run(shutdown);

    // std::process::exit()는 Drop을 건너뜀.
    // killer를 명시적으로 drop해 kill loop 자식 프로세스를 반드시 회수한다.
    drop(killer);

    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("\n[FAIL] {}", DisplayError(e));
            std::process::exit(1);
        }
    }
}

// ── Phase 3: 연결 및 정보 출력 ──────────────────────────────────────────────

fn run(_shutdown: Arc<AtomicBool>) -> Result<(), SdkError> {
    // 1. SDK 초기화
    let session = SdkSession::new(0)?;
    println!("[OK] SDK version: 0x{:08X}", session.version());

    // 2. 카메라 탐색 (5초)
    println!("[..] Scanning for cameras (5s)...");
    let cameras = CameraEnumerator::new(&session, 5).map_err(|e| {
        eprintln!("[!!] Enumeration failed (CrError_Adaptor_Create = USB 선점). Checklist:");
        eprintln!("       1. Camera USB mode → [PC Remote]");
        eprintln!("       2. Quit: Image Capture, Photos, Android File Transfer");
        eprintln!("          (Android File Transfer Agent는 백그라운드에서도 선점함)");
        eprintln!("       3. Run: pkill -KILL 'Android File Transfer Agent'");
        e
    })?;

    let count = cameras.count();
    println!("[OK] Found {} camera(s)", count);

    if count == 0 {
        eprintln!("[!!] No cameras detected. Is the camera on and in PC Remote mode?");
        return Ok(());
    }

    // 3. 발견된 모든 카메라 정보 출력
    for (i, info) in cameras.list_all()?.iter().enumerate() {
        println!(
            "     [{i}] name='{}' model='{}' USB_PID=0x{:04X} status={} transport={}",
            info.name, info.model, info.usb_pid, info.connection_status,
            if info.ssh_support { "WiFi/SSH" } else { "USB" }
        );
    }

    // 4. 연결 모드 결정 (자동 감지)
    let cam_info = cameras.get(0)?;
    let cam_ptr  = cameras.camera_ptr(0)?;

    let mode: ConnectMode<'_>;
    // fingerprint / password 소유권을 루프 밖에 유지
    let _fp_buf: Vec<u8>;
    let _pw_buf: String;

    if cam_info.ssh_support {
        println!("[..] WiFi/SSH 카메라 감지 — fingerprint 취득 중 (네트워크 왕복)...");
        _fp_buf = cameras.get_fingerprint(0)?;

        if _fp_buf.is_empty() {
            eprintln!("[WW] Fingerprint를 가져오지 못했습니다. 빈 fingerprint로 시도합니다.");
        } else {
            // SHA256 hex 형식으로 출력 (OpenSSH known_hosts 스타일)
            let hex: String = _fp_buf.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":");
            println!("[OK] SSH fingerprint: {hex}");
        }

        // 비밀번호: 환경변수 CAMERA_PASSWORD 우선, 없으면 stdin
        _pw_buf = if let Ok(pw) = std::env::var("CAMERA_PASSWORD") {
            println!("[..] CAMERA_PASSWORD 환경변수 사용");
            pw
        } else {
            eprint!("[..] SSH 비밀번호 입력 (카메라 설정 화면 비밀번호): ");
            let mut pw = String::new();
            std::io::stdin().read_line(&mut pw).unwrap_or(0);
            pw.trim_end_matches(['\n', '\r']).to_owned()
        };

        mode = ConnectMode::Wifi {
            password: &_pw_buf,
            fingerprint: &_fp_buf,
            pairing_name: "CrSDK-Rust",
        };
        println!("[..] Connecting via WiFi/SSH (timeout 10s)...");
    } else {
        _fp_buf = Vec::new();
        _pw_buf = String::new();
        mode = ConnectMode::Usb;
        println!("[..] Connecting via USB (timeout 10s)...");
    }

    let conn = Camera::connect(&session, cam_ptr, Duration::from_secs(10), mode)
        .map_err(|e| {
            eprintln!("[!!] Connect failed. Is another app using the camera?");
            e
        })?;

    println!("[OK] Connected. device_handle=0x{:016X}", conn.device_handle());

    // 5. 연결 성공 확인 — Phase 4(LiveView), Phase 5(셔터)는 별도 테스트
    println!();
    println!("Phase 3 PASSED: SDK init + enumerate + connect OK.");
    println!("Next: Phase 4 (LiveView) — uncomment LiveView block below.");

    // ── Phase 4 블록 (LiveView) ─────────────────────────────────────────────
    // 연결 확인 후 아래 주석을 해제하여 테스트
    /*
    use crsdk::callback::CameraEvent;
    use crsdk::liveview::LiveViewStream;
    let shutdown = _shutdown;

    println!("\n[..] Waiting 2s for camera LiveView startup...");
    std::thread::sleep(Duration::from_secs(2));

    let mut conn = conn; // need mut for conn.events()

    // Helper closure: allocate LiveViewStream, treating Unavailable as non-fatal.
    // Returns None if LiveView is not ready yet (caller should break or retry).
    let make_lv = |handle: i64| -> SdkResult<Option<LiveViewStream>> {
        match LiveViewStream::new(handle) {
            Ok(lv) => Ok(Some(lv)),
            Err(SdkError::LiveViewUnavailable) => Ok(None),
            Err(e) => Err(e),
        }
    };

    let mut lv = match make_lv(conn.device_handle())? {
        Some(lv) => {
            println!("[OK] LiveView ready.");
            lv
        }
        None => {
            eprintln!("[!!] LiveView unavailable (check camera display/mode settings).");
            return Ok(());
        }
    };

    println!("[..] Running LiveView event loop (Ctrl+C to stop)...");
    let mut frame_count = 0u64;

    // ── 16 ms event loop ────────────────────────────────────────────────────
    // recv_timeout(16 ms) ≈ 60 fps ceiling without busy-wait:
    //   • Timeout branch: try to fetch a LiveView frame.
    //   • Event branch  : handle async SDK notifications (property changes,
    //     download-complete, warnings, disconnects).
    //
    // This avoids a separate liveview-poll thread while keeping the main
    // thread responsive to SDK events and Ctrl+C.
    loop {
        if shutdown.load(Ordering::Relaxed) {
            println!("\n[..] Shutdown signal — exiting loop.");
            break;
        }

        match conn.events().recv_timeout(Duration::from_millis(16)) {
            Err(_timeout) => {
                // No SDK event arrived — use this slot to fetch a LV frame.
                match lv.fetch_frame() {
                    Ok(frame) if !frame.is_empty() => {
                        frame_count += 1;
                        if frame_count <= 5 || frame_count % 30 == 0 {
                            let path = format!("liveview_{frame_count:06}.jpg");
                            if let Err(e) = std::fs::write(&path, &frame) {
                                eprintln!("[!!] write {path}: {e}");
                            } else {
                                println!("  frame {frame_count}: {} bytes → {path}",
                                         frame.len());
                            }
                        }
                    }
                    Ok(_) => { /* empty frame — camera not ready yet, skip */ }
                    Err(SdkError::LiveViewUnavailable) => {
                        eprintln!("[!!] LiveView lost.");
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(CameraEvent::PropertyChanged) => {
                // Properties changed — good time to re-read exposure settings
                // if needed.  For now just log at verbose level.
            }

            Ok(CameraEvent::LvPropertyChanged) => {
                // LvPropertyChanged can signal a resolution or aspect-ratio change.
                // The camera may now be sending larger frames than our buffer can hold.
                // Buffer overflow will occur if we keep fetching with the old block.
                //
                // Fix: drop the old LiveViewStream (frees the block) and reallocate
                // with the new buffer size reported by liveview_get_buffer_size.
                drop(lv);
                // Give the camera ~200ms to stabilise the new format.
                std::thread::sleep(Duration::from_millis(200));
                lv = match make_lv(conn.device_handle())? {
                    Some(new_lv) => {
                        println!("[..] LiveView buffer reallocated after format change.");
                        new_lv
                    }
                    None => {
                        eprintln!("[!!] LiveView unavailable after property change.");
                        break;
                    }
                };
            }

            Ok(CameraEvent::DownloadComplete { filename, kind }) => {
                println!("[OK] Download complete: kind={kind} path={filename}");
            }

            Ok(CameraEvent::Disconnected { error }) => {
                eprintln!("[!!] Camera disconnected (error=0x{error:08X})");
                break;
            }

            Ok(CameraEvent::Warning(code)) => {
                eprintln!("[WW] SDK warning 0x{code:08X}");
            }

            Ok(CameraEvent::WarningExt { code, p1, p2, p3 }) => {
                eprintln!("[WW] SDK warning-ext 0x{code:08X} ({p1},{p2},{p3})");
            }

            Ok(CameraEvent::Error(code)) => {
                eprintln!("[!!] SDK error 0x{code:08X}");
                break;
            }

            Ok(CameraEvent::Connected { .. }) => {
                // Reconnect notification — already connected, ignore.
            }
        }
    }

    println!("Phase 4 PASSED: LiveView OK ({frame_count} frames captured).");
    */

    // ── Phase 5 블록 (셔터) ─────────────────────────────────────────────────
    // LiveView 확인 후 아래 주석을 해제하여 테스트
    /*
    use crsdk::shutter;
    println!("\n[..] Triggering shutter (Manual mode + MF recommended)...");
    shutter::capture(conn.device_handle())?;
    println!("[OK] Shutter OK.");
    println!("Phase 5 PASSED: Shutter OK.");
    */

    // Drop 순서: conn → deactivate+disconnect+release, session → sdk_release
    Ok(())
}
