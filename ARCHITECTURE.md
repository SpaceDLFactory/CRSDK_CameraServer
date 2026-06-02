# ARCHITECTURE — CrSDK Rust Wrapper + 테더링 서버

전체 맥락용 문서. lib(`crsdk`) + 서버(`crsdk_server`) + 웹 UI를 한눈에.
(lib 내부 설계 이력은 `design_v5.md`, 단계별 로드맵은 `PLAN.md` 참조.)

## 1. 큰 그림

```
[Sony A7C] ──USB(PTP)── [Mac: crsdk_server] ──HTTP/SSE/WiFi── [브라우저/폰: Tether Console UI]
```

단일 카메라를 동기 제어하는 Rust FFI 래퍼 위에, axum HTTP/SSE 서버를 얹어
브라우저/폰에서 연결·라이브뷰·노출 제어·촬영·저장을 원격으로 한다.

## 2. 워크스페이스 구조

```
crsdk_rust_wrapper/                  (Cargo workspace)
├── Cargo.toml                       [workspace] + crsdk lib(package)
├── src/                             ── crsdk lib (safe FFI 래퍼) ──
│   ├── ffi.rs            bindgen 생성 바인딩 (pub(crate))
│   ├── error.rs          CrErrorCode, SdkError, is_error/is_warning
│   ├── session.rs        SdkSession — SDK init/release (RAII)
│   ├── enumerate.rs      CameraEnumerator — 카메라 탐색
│   ├── callback.rs       DeviceCallback + CameraEvent (std::mpsc)
│   ├── connection.rs     Camera (RAII Drop 체인, take_events, set_save_info)
│   ├── liveview.rs       LiveViewStream — JPEG 프레임 fetch
│   ├── shutter.rs        capture (MF) / capture_af (AF S1 시퀀스)
│   └── properties.rs     get/set + 속성 코드·값 상수
├── wrapper/wrapper.{h,cpp}          C++ SDK → 순수 C ABI shim
├── build.rs                         cc(libwrapper.a) + bindgen + CrAdapter symlink + rpath
└── crsdk_server/                    ── axum HTTP/SSE 서버 (bin) ──
    ├── src/main.rs                  핸들러·AppState·어댑터
    └── web/index.html               단일 페이지 UI (B·Tether Console)
```

## 3. 레이어 (FFI 3단)

```
Sony C++ SDK (네임스페이스·vtable)
  └─ wrapper.cpp   순수 C 함수 + opaque void*  (예외/C++타입 경계 차단, std::atomic 콜백)
      └─ bindgen → ffi.rs   (unsafe extern "C")
          └─ src/*.rs   안전 Rust (RAII, Result, Send 경계)
              └─ crsdk_server   비즈니스/HTTP
```

## 4. 서버 상태 & 동시성

```rust
AppState {
    camera:    Arc<tokio::Mutex<Option<CameraCell>>>,  // CameraCell = unsafe Send newtype
    save_path: Arc<tokio::Mutex<String>>,
    events_tx: broadcast::Sender<String>,              // JSON 직렬화 이벤트 fan-out
}
```

- **세션 'static화**: `OnceLock<SdkSession>` → `Camera<'static>`
- **SDK는 동기(blocking)** → 모든 SDK 호출은 `spawn_blocking`으로 격리 (tokio 워커 보호)
- **락 규칙**: `.await` 전에 handle만 꺼내고 락 해제 → 블로킹 작업 중 락 미보유

## 5. HTTP/SSE 엔드포인트

| 메서드 | 경로 | 역할 |
|--------|------|------|
| GET | `/api/status` | 연결상태·model·handle·save_path |
| POST | `/api/connect` `/disconnect` | 연결 (PriorityKey=PCRemote + set_save_info 자동) |
| GET | `/api/properties` | ISO/SS/Av/EV/WB/Drive/Metering/FileType/Focus/Save 현재값+allowed |
| POST | `/api/property` `{code,value}` | 속성 쓰기 (Fetch-Modify-Set) |
| POST | `/api/shutter` | focus mode 감지 → MF: capture / AF: capture_af |
| POST | `/api/savepath` `{path}` | 저장 폴더 변경 |
| GET | `/events` (SSE) | DownloadComplete·PropertyChanged·Disconnected·Error |
| GET | `/lv` (MJPEG) | LiveView 영상 스트림 |
| GET | `/web/*` | 정적 UI |

## 6. 핵심 데이터 흐름

```
연결:   POST /connect → spawn_blocking[enumerate→Connect→PriorityKey→set_save_info]
                      → CameraCell 저장 + take_events()→어댑터 spawn

이벤트: SDK 콜백스레드 →(std mpsc)→ Camera.events →take_events→ 어댑터(spawn_blocking)
                      →(JSON)→ broadcast →subscribe→ SSE → 브라우저 EventSource

라이브뷰: GET /lv → spawn_blocking[LiveViewStream 16ms fetch] →(mpsc 2)→ ReceiverStream
                  → multipart/x-mixed-replace → <img>

속성:   GET /properties(2s 폴링) + SSE PropertyChanged(즉시) → fillSelect dropdown
        change → POST /property → spawn_blocking[set] → 재폴링

셔터:   POST /shutter → spawn_blocking[focus mode read → MF capture / AF S1+capture]
        → PC저장 시 SDK 다운로드 → OnCompleteDownload → SSE → "저장됨" 토스트
```

## 7. RAII / 안전성 불변식

- **Drop 체인**(Camera): deactivate_callback → disconnect → release_device → destroy_callback
- **콜백 함수포인터 std::atomic** → Drop 시 SDK 백그라운드 스레드가 null 만나 no-op (UAF 방지)
- **USB 억제기**: ptpcamerad 50ms kill loop (Drop이 회수)
- **경고≠에러**: `is_warning()`로 CrWarning(0x2xxxx)을 빈 결과로 처리 (LiveView 스트림 유지)
- **Ctrl+C**: ctrlc + graceful_shutdown → Drop 보장

## 8. 속성 코드 (헤더 enum을 정확히 파싱한 값 — 순차 아님)

| 속성 | 코드 | 비고 |
|------|------|------|
| S1 (반누름) | 0x0001 | AF lock 트리거 |
| FNumber | 0x0100 | f값 = value/100 |
| ExposureBiasCompensation (EV) | 0x0101 | 부호 1/1000 EV |
| ShutterSpeed | 0x0103 | (num<<16)\|den |
| IsoSensitivity | 0x0104 | 0xFFFFFF=AUTO |
| ExposureProgramMode | 0x0105 | A7C는 물리 다이얼(읽기 전용) |
| FileType | 0x0106 | 1=JPEG 2=RAW 3=RAW+JPEG |
| WhiteBalance | 0x0108 | |
| MeteringMode | 0x010A | |
| DriveMode | 0x010E | |
| FocusMode | 0x0109 | 1=MF 2=AF-S 3=AF-C |
| FocusIndication | 0x0707 | AF 합초 결과(미사용) |
| StillImageStoreDestination | 0x0119 | 1=PC 2=SD 3=PC+SD |
| PriorityKeySettings | 0x011A | 2=PCRemote(쓰기 권한) |

## 9. 빌드 / 실행

```bash
# clang 21(시스템 Xcode)면 SDKROOT 불필요. (외장 Xcode15였을 땐 SDKROOT로 14.0 SDK 지정 필요)
cargo build -p crsdk_server
pkill -KILL ptpcamerad        # 서버 내장 억제기가 처리하지만 부팅 전 1회 권장
cargo run -p crsdk_server     # http://localhost:8080/web/index.html
```

## 10. 알려진 부채 / 다음 후보

- LiveView 단일 클라이언트 / 해상도 변경 시 버퍼 재할당 미처리
- USB 억제기 example/server 중복 → lib `crsdk::platform`으로 DRY 여지
- AF 셔터는 시간 기반(500ms). 합초 실패율 높으면 FocusIndication(0x0707) 폴링으로 전환
- ExposureProgramMode(PASM)는 A7C 물리 다이얼이라 SDK 쓰기 불가 (allowed 비어 비활성)
- 미구현: 촬영 미리보기, 배터리/저장공간 표시, 재연결, RAW 압축/JPEG 품질, LV 피킹/확대
