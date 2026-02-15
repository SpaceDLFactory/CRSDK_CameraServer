# sonyCRSDK rust bind

Sony Camera Remote SDK(macOS)를 Rust 언어로 제어하기 위한 FFI(Foreign Function Interface) 래퍼 프로젝트입니다.  
C++ 기반의 SDK 인터페이스를 Rust의 안전한 에코시스템 내에서 활용할 수 있도록 설계되었으며, SDL Factory 프로젝트의 핵심 엔진으로 활용됩니다.

---

## 🚀 Key Features

- **Safe FFI Binding**  
  `bindgen`을 사용하여 Sony SDK의 타입과 상수를 Rust 코드로 자동 생성합니다.
- **C-Shim Bridge**  
  Rust에서 직접 호출하기 까다로운 C++ 인터페이스(`Namespace: SCRSDK`)를 `extern "C"` 함수로 래핑하여 안정성을 확보했습니다.
- **macOS Optimized**  
  Apple Silicon(M1/M2/M3) 환경의 `.dylib` 동적 라이브러리 링크 및 `rpath` 설정을 지원합니다.
- **Rust 2024 Edition**  
  최신 Rust 표준을 사용하여 견고한 빌드 시스템을 구축했습니다.

---

## 🛠 Prerequisites

본 프로젝트를 빌드하고 실행하기 위해 다음 구성 요소가 필요합니다.

1. **Sony Camera Remote SDK**  
   Sony Developer World에서 다운로드한 macOS용 SDK (**v2.01.00 이상**)
2. **LLVM/Clang**  
   `bindgen`이 C++ 헤더를 분석하기 위해 필요합니다.
   ```bash
   brew install llvm
   ```
3. **Rust Toolchain**  
   Rust 2024 Edition을 지원하는 최신 Rust 버전

---

## 📂 Project Structure

```text
crsdk_rust_wrapper/
├── Cargo.toml            # 프로젝트 의존성 (thiserror 2.0.18, bindgen 0.72.1)
├── build.rs              # C++ 컴파일 및 바인딩 자동 생성 스크립트
├── wrapper.cpp           # C++ to C 징검다리 코드 (C-Shim)
├── wrapper.hpp           # SDK 헤더 진입점 및 Namespace 정의
├── src/
│   └── main.rs           # SDK 초기화 및 테스트 로직
└── CrSDK_v2.01.00_.../   # Sony SDK 원본 폴더 (사용자 직접 배치)
```

---

## ⚙️ Setup & Build

1. **SDK 배치**  
   다운로드한 SDK를 프로젝트 루트에 배치합니다.
2. **경로 확인**  
   `build.rs` 내의 `sdk_base` 변수값이 실제 폴더명(예: `CrSDK_v2.01.00_20260203a_Mac`)과 일치하는지 확인합니다.
3. **빌드**
   ```bash
   cargo build
   ```
4. **런타임 라이브러리 경로 설정**  
   macOS 환경에서 동적 라이브러리 로드를 위해 실행 시 경로 지정이 필요합니다.
   ```bash
   export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:$(pwd)/CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk/
   cargo run
   ```

---

## 💻 Usage Example (`src/main.rs`)

```rust
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// build.rs에서 생성된 바인딩 파일 포함
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn main() {
    println!("--- SDL Factory: Sony SDK 연동 테스트 ---");

    unsafe {
        // 1. SDK 버전 확인
        let version = get_sdk_version();
        println!("SDK Version: {}", version);

        // 2. SDK 초기화
        if sdk_init(0) {
            println!("✅ Sony SDK 초기화 성공!");

            // TODO: 카메라 탐색(EnumCameraObjects) 로직 구현 예정

            // 3. SDK 리소스 해제
            if sdk_release() {
                println!("✅ Released!");
            }
        } else {
            println!("❌ SDK 초기화 실패");
        }
    }
}
```

---

## 🗺 Roadmap

- [x] SDK Environment Setup & Linker Configuration (macOS)
- [x] C-Shim Bridge for C++ Namespace Handling (`SCRSDK`)
- [x] Basic SDK Initialization & Version Check
- [ ] Camera Device Enumeration (`EnumCameraObjects`)
- [ ] LiveView Stream Data Handling for YOLO/ByteTrack
- [ ] Remote Shutter & PTZF (Pan/Tilt/Zoom/Focus) Automation
- [ ] SDL Factory: Deep Learning based Astronomical Tracking System

---

## ⚖️ License

This project is licensed under the **MIT License**.

> Note: Sony SDK 관련 헤더 및 라이브러리 파일의 저작권은 Sony에 있습니다.