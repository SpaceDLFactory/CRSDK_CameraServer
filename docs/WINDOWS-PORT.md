# Windows 포팅 — 작업 가이드 (windows-prep 브랜치)

> 이 브랜치는 **크로스플랫폼 구조화(cfg 스캐폴딩)** 가 끝난 상태다. macOS 빌드는 그대로 통과.
> Windows 머신에서 아래 **빈칸(TODO)** 만 채우면 된다. 전략: **B(본인 Windows 머신, 빌드+실카메라)**.

## 0. 시작 방법
```
git clone <repo> && cd <repo>
git checkout windows-prep
```
그 다음 §1 준비 → §2 빈칸 채우기 → §3 검증 순서.

---

## 1. Windows 머신 사전 준비물
1. [ ] **Rust** — rustup, 기본 타깃 `stable-x86_64-pc-windows-msvc` (`rustup default stable-msvc`)
2. [ ] **Visual Studio Build Tools** — "Desktop development with C++" (MSVC + Windows SDK). `cc`/링크 필수
3. [ ] **LLVM (clang/libclang)** — bindgen용. `LIBCLANG_PATH` 환경변수 (예: `C:\Program Files\LLVM\bin`)
4. [ ] **Windows용 Sony CrSDK** — Developer World에서 다운로드, 압축 해제
5. [ ] **Git**
6. [ ] 카메라 A7C USB 연결 (+ 필요 시 드라이버 — §2.4)

---

## 2. 채울 빈칸 (코드의 `TODO(windows)`)

> 각 항목은 코드에 마커가 있다: `grep -rn "TODO(windows)"`

### 2.1 SDK 경로 — `build.rs:~11`
```rust
manifest.join("CrSDK_Win") // ← 실제 압축 해제한 Windows SDK 폴더명/위치로
```
- Windows SDK의 헤더 경로(`.../app/CRSDK`)와 lib 경로(`.../external/crsdk`)가 macOS와 같은 하위구조인지 확인. 다르면 `sdk_include`/`sdk_lib` 조합도 맞춘다.
- `Cr_Core.lib`(import lib)가 `sdk_lib`에 있는지 확인 (`rustc-link-lib=dylib=Cr_Core` 가 이를 찾음).

### 2.2 CrAdapter 플러그인 배치 — `build.rs:~83` (`#[cfg(windows)]` 블록)
- macOS는 NSBundle 기준 `Contents/Frameworks/CrAdapter`에 심링크. **Windows엔 NSBundle 없음.**
- **실측**: Windows SDK의 RemoteSampleApp이 USB 플러그인(`Cr_PTP_USB.dll`)을 어디서 찾는가? (대개 **exe와 같은 폴더의 `CrAdapter\`**).
- 확정되면 그 위치로 `CrAdapter` DLL들을 **복사**(심링크 대신). `dev` 빌드는 `target/debug/` 옆, 배포는 exe 옆.

### 2.3 CrChar(문자열) 인코딩 — lib `src/enumerate.rs` 등
- **확정 사실**: `CrTypes.h`에서 Windows는 `CrChar = wchar_t`(UTF-16), macOS는 `char`(UTF-8).
- `read_cchar`(enumerate.rs)와 모델/이름/연결타입 getter들이 macOS는 `*const c_char`(UTF-8) 가정.
- **Windows**: `GetModel()`/`GetName()` 등이 `wchar_t*` 반환 → **UTF-16→UTF-8 변환** 필요.
- 권장: wrapper.cpp 쪽에서 Windows일 때 `WideCharToMultiByte`로 UTF-8 변환해 반환(현재 `get_property_string`이 UTF-16→ASCII 하듯), lib 시그니처는 유지. 또는 lib에 `#[cfg(windows)]` 변환 분기.

### 2.4 USB 억제기 — `main.rs:~79` (현재 비-macOS no-op)
- macOS의 ptpcamerad 억제는 Windows에 없음. **실측**: A7C가 Windows에서 PTP/USB로 잡히려면 드라이버가 필요한지(WinUSB/libusb, Zadig 등) SDK Readme Windows 섹션 확인.
- 보통 SDK가 알아서 처리 → no-op 유지 가능. 문제 시 드라이버 안내 추가.

### 2.5 단일 인스턴스 — `main.rs:~1200` (현재 비-unix no-op)
- 권장: **named mutex**(`CreateMutexW` + `GetLastError()==ERROR_ALREADY_EXISTS`) 로 중복 실행 방지. 또는 `tasklist`/`taskkill`.
- `windows` crate를 `[target.'cfg(windows)'.dependencies]`로 추가해 구현.

### 2.6 (이미 분기됨, 손댈 것 없음)
- 브라우저 오픈: `cmd /C start` (main.rs) ✅
- cc 플래그: MSVC `/GR-` 분기 ✅ (예외 끄기는 SDK 헤더가 예외 쓰면 빼야 할 수 있음 — 빌드 에러 시 조정)
- rpath: Windows는 건너뜀 ✅ → DLL은 exe 옆/PATH

---

## 3. 검증 순서 (매 단계 컴파일)
1. [ ] `cargo build -p crsdk_server` — 빌드 통과까지 §2.1→2.2→2.3 순서로 에러 잡기
2. [ ] 실행 후 `Cr_Core.dll`·`CrAdapter\*.dll`이 exe 옆에 있어야 로드됨 (없으면 `0xc000007b`/not found)
3. [ ] 카메라 USB 연결 → `GET /api/_debug/enum` 으로 발견 확인 (모델명이 깨지면 §2.3 인코딩 문제)
4. [ ] 연결·라이브뷰·촬영 스모크 테스트
5. [ ] 단일 인스턴스(§2.5)·종료 동작 확인

## 4. 패키징 (1차)
- `TetherMoon-win-x64.zip` = `crsdk_server.exe` + 모든 DLL(`Cr_Core.dll` + `CrAdapter\` + `libusb-1.0.dll` 등) + `web\` + README.txt
- 미서명 → SmartScreen "추가 정보 → 실행" 안내. 아이콘은 `.ico`(달 사진)로 별도.

## 5. 마무리
- [ ] README에 Windows 설치 섹션
- [ ] main 브랜치로 merge (검증 완료 후)
- [ ] (선택) GitHub Actions windows-latest 빌드 — 단, SDK를 CI에 공개로 못 올리는 문제 고려

---

**현재 브랜치 상태**: 플랫폼 의존 4곳 cfg 격리 완료(USB억제기·단일인스턴스·브라우저·build.rs). macOS 빌드/실행 검증됨. 남은 건 위 §2의 Windows 실측 5건 + 패키징.
