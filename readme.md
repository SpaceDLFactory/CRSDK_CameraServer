# sonyCRSDK rust bind

Sony Camera Remote SDK(macOS)를 **Rust로 안전하게 래핑**하고, 그 위에 **브라우저 테더링 서버**를 올린 프로젝트입니다.
C++ 기반 SDK를 `extern "C"` C-shim + `bindgen`으로 Rust에 연결하고, `axum` HTTP/SSE 서버가 카메라를 노출해
**웹/폰 브라우저에서 노출·포커스·촬영·라이브뷰**를 원격 제어합니다.

> ## ⚠️ 대상 기기: Sony A7C (ILCE-7C) 전용
> 이 툴은 **ILCE-7C 한 대로만 개발·검증**되었습니다. 다른 바디는 테스트되지 않았으며,
> A7C가 노출하지 않는 기능(자이로/CreativeLook/벌브타이머/AF영역 device property 등)은
> 코드에 남아 있어도 이 바디에선 동작하지 않습니다. 멀티바디 지원은 향후 과제입니다.

---

## 🚀 기능

- **Safe FFI**: `bindgen`으로 SDK 타입/함수 자동 바인딩 + `wrapper/`의 pure-C shim(`SCRSDK` 네임스페이스 우회)
- **테더링 서버**(`crsdk_server`, axum/tokio): 자동 연결·재연결, 모든 SDK 호출 `spawn_blocking` 격리
- **웹 UI**(단일 페이지): MJPEG 라이브뷰 + 포커스 피킹, 노출/색(ISO·셔터·조리개·EV·WB·켈빈 슬라이더·측광·드라이브·파일포맷·JPEG품질·Picture Profile)
- **촬영**: 단발·연사·동영상·취소, **장노출**(고정 1"~30" / BULB / 소프트웨어 벌브 타이머), **소프트웨어 인터벌(타임랩스)**
- **포커스**: MF NearFar 슬라이더, AF 포인트(라이브뷰 클릭, 좌표 보정) + 박스 크기 S/M/L, 반셔터(S1)
- **저장/상태**: PC 저장(경로·접두사), 촬영 미리보기, 배터리·남은 컷, 라이브뷰 회전 토글
- **안전 종료**: SIGTERM/SIGINT graceful shutdown으로 카메라 세션 클린 해제(재연결 FailBusy 방지)

자세한 기능 현황은 [`STATUS.md`](STATUS.md), 구조는 [`ARCHITECTURE.md`](ARCHITECTURE.md) 참조.

---

## 🛠 빌드 전제

1. **Sony Camera Remote SDK** (macOS, v2.01.00) — Sony Developer World에서 받아 프로젝트 루트에 `CrSDK_v2.01.00_20260203a_Mac/`로 배치. **저작권 Sony, 본 저장소에 미포함**(.gitignore).
2. **LLVM/Clang** (`bindgen`용): `brew install llvm`
3. **Rust** (edition 2021)

## 📂 구조

```text
crsdk_rust_wrapper/
├── Cargo.toml            # workspace (crsdk lib + crsdk_server)
├── build.rs              # cc로 wrapper 컴파일 + bindgen 바인딩 생성
├── wrapper/              # wrapper.{h,cpp} — pure-C shim
├── src/                  # safe Rust: session/enumerate/connection/liveview/shutter/control/properties/callback/error
├── crsdk_server/         # axum 서버 + web/index.html (브라우저 UI)
└── CrSDK_v2.01.00_.../   # Sony SDK (사용자 직접 배치, 미포함)
```

## ⚙️ 빌드 & 실행

```bash
# SDK dylib 런타임 경로
export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:$(pwd)/CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk/

# 테더링 서버 (브라우저 UI)
cargo run -p crsdk_server      # http://localhost:8080/web/index.html

# FFI 동작 확인용 예제
cargo run --bin crsdk_example
```

macOS의 `ptpcamerad`가 USB 카메라 접근을 방해하므로 서버가 부팅 시 억제합니다(정상 동작).

---

## 🌙 첫 작품

이 툴로 찍은 첫 사진. ILCE-7C + FE 100-400 GM, 무보정.

![first moon](gallery/first-moon.jpg)

> © neko.kim.film (김괭필름)

---

## ⚖️ License

MIT License.

> Sony SDK 관련 헤더·라이브러리·문서의 저작권은 Sony에 있으며 본 저장소에 포함되지 않습니다.
