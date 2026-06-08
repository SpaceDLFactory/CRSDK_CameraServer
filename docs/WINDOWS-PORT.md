# Windows 포팅 — 작업 가이드 (windows-prep 브랜치)

> **상태(2026-06): Windows 빌드 + 서버 실행 + SDK init + 웹 UI 서빙 검증 완료.**
> 남은 단 하나: **실제 A7C를 Windows에 PC Remote 모드로 연결한 live 테스트**(enum/connect/촬영 + §2.3 문자열 인코딩 실측). 단일 인스턴스 named mutex(§2.5)는 선택.
>
> 검증 환경(실제 머신): Rust 1.96 msvc, VS BuildTools `F:\BuildTools`, LLVM `F:\LLVM\bin`(libclang), Win SDK는 `CrSDK_Win/`(gitignore). 빌드:
> ```
> set "LIBCLANG_PATH=F:\LLVM\bin"   # 따옴표 필수 — trailing space 들어가면 bindgen이 못 찾음
> cargo build -p crsdk_server
> ```

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

### 2.1 SDK 경로 — `build.rs` ✅ 완료
- `is_windows`면 `CrSDK_Win/{app/CRSDK, external/crsdk}` 사용(Mac는 `RemoteCli/` 하위). `Cr_Core.lib` 확인됨.
- 주의: Win64 SDK zip은 중첩(`RemoteCli.zip` 안에 또 있음). `RemoteCli.zip`만 풀면 `RemoteCli/` 없이 바로 `app/`,`external/`.

### 2.2 CrAdapter 플러그인 배치 — `build.rs` `#[cfg(windows)]` ✅ 완료
- OUT_DIR에서 `target\<profile>` 파생 → top-level `*.dll`(Cr_Core, monitor_protocol*)은 exe 옆에, `CrAdapter\*.dll`(Cr_PTP_USB/IP, libssh2, libusb-1.0)은 exe 옆 `CrAdapter\`로 **복사**. (절대 하드코딩 경로 없음.)

### 2.3 CrChar(문자열) 인코딩 — ⚠️ **live 카메라로 실측 필요**
- **정정**: `CrTypes.h`의 `CrChar = wchar_t`는 **`UNICODE`/`_UNICODE` 매크로가 정의됐을 때만**. cc-rs(cl.exe)는 이를 정의하지 않으므로 우리 빌드에선 `CrChar = char`(CrAChar) → `GetName()`/`GetModel()`이 `const char*`로 컴파일됨(= macOS와 동일, 변환 불필요).
- **남은 위험**: 정작 사전컴파일된 `Cr_Core.dll`이 UNICODE(wchar) 빌드라면, DLL이 돌려주는 버퍼는 UTF-16인데 우리는 char로 읽어 깨질 수 있음("I\0L\0C\0E\0…" 형태). → **실카메라로 모델명 확인**해서 정상이면 끝, 깨지면 wrapper에서 `UNICODE` 정의 + `WideCharToMultiByte` 변환으로 전환.

### 2.4 USB 억제기 — `main.rs` 비-macOS no-op ✅ (유지)
- macOS의 ptpcamerad 억제는 Windows에 없음 → no-op 유지. WARN 로그는 macOS에서만 뜨도록 cfg 게이팅함.
- A7C는 Windows에서 카메라 메뉴 **USB 연결모드 = PC Remote** 여야 CrSDK가 enum함(Imaging Edge 가상 웹캠 디바이스와 무관).

### 2.5 단일 인스턴스 — `main.rs:~1200` (현재 비-unix no-op)
- 권장: **named mutex**(`CreateMutexW` + `GetLastError()==ERROR_ALREADY_EXISTS`) 로 중복 실행 방지. 또는 `tasklist`/`taskkill`.
- `windows` crate를 `[target.'cfg(windows)'.dependencies]`로 추가해 구현.

### 2.6 (이미 분기됨, 손댈 것 없음)
- 브라우저 오픈: `cmd /C start` (main.rs) ✅
- cc 플래그: MSVC `/GR-` 분기 ✅ (예외 끄기는 SDK 헤더가 예외 쓰면 빼야 할 수 있음 — 빌드 에러 시 조정)
- rpath: Windows는 건너뜀 ✅ → DLL은 exe 옆/PATH

---

## 3. 검증 순서 (매 단계 컴파일)
1. [x] `cargo build -p crsdk_server` — 빌드 통과 (wrapper.cpp `__builtin_memcpy`→`memcpy`, `LIBCLANG_PATH` 따옴표 이슈 해결)
2. [x] 실행 → `Cr_Core.dll`·`CrAdapter\*.dll` 자동 복사·로드, SDK init OK, 8080 리슨, 웹 UI 서빙
3. [ ] **카메라 USB(PC Remote) 연결** → enum 발견 + 모델명 안 깨지는지(§2.3) 확인
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

**현재 브랜치 상태**: Windows에서 **빌드+실행+SDK init+웹서빙 검증 완료**(실측 머신). 코드 포팅 사실상 종료. 남은 건 (a) 실 A7C live 테스트(§3.3~3.5, §2.3 인코딩 확인), (b) 단일 인스턴스 named mutex(§2.5, 선택), (c) 패키징(§4)·README(§5)·main merge.
