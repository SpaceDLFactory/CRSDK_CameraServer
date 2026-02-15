# sonyCRSDK_rust_bind


Sony Camera Remote SDK(macOS)를 위한 Rust FFI 래퍼 프로젝트입니다. 
C++ 기반의 SDK를 Rust에서 안전하고 효율적으로 제어할 수 있는 기반을 제공합니다.

## 🚀 Key Features
* **Safe FFI Binding**: `bindgen`을 이용한 Sony SDK 자동 바인딩 생성.
* **C-Shim Bridge**: 복잡한 C++ 인터페이스를 `extern "C"`로 래핑하여 Rust 호출 최적화.
* **macOS Optimized**: Apple Silicon(M1/M2/M3) 및 macOS 환경의 동적 라이브러리(`.dylib`) 링크 설정 완료.

## 🛠 Prerequisites
본 프로젝트를 빌드하려면 다음 요소가 필요합니다.

1. **Sony Camera Remote SDK**: [Sony Developer World](https://support.d-imaging.sony.co.kr/app/sdk/en/index.html)에서 다운로드한 SDK 폴더.
2. **LLVM/Clang**: `bindgen` 실행을 위해 시스템에 설치되어 있어야 합니다.
   ```bash
   brew install llvm