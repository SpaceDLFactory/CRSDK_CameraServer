// crsdk_server/build.rs
//
// crsdk(lib)의 build.rs가 emit한 `rustc-link-arg`는 crsdk crate 자체에만
// 적용되어 crsdk_example 같은 동일 crate 안의 bin에만 rpath가 새겨진다.
// crsdk_server는 다운스트림 crate이므로 직접 rpath를 박아야 한다.

use std::env;
use std::path::PathBuf;

fn main() {
    // rpath는 Mach-O/ELF 전용(macOS). Windows는 rpath 개념이 없고(DLL은 exe 옆/PATH),
    // 링크용 search/lib 지시는 crsdk lib의 build.rs가 emit해 다운스트림에 전파되므로
    // 여기선 아무것도 안 해도 된다.
    let is_windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if !is_windows {
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        // crsdk_server 는 crsdk 한 단계 아래 → SDK lib 는 ../CrSDK_.../external/crsdk
        let sdk_lib = manifest
            .parent()
            .expect("crsdk_server has no parent dir")
            .join("CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk");
        assert!(sdk_lib.exists(), "SDK lib dir not found: {sdk_lib:?}");
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg={}", sdk_lib.display());
    }
}
