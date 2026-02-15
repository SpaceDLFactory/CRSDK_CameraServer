use std::env;
use std::path::PathBuf;
use bindgen;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let sdk_base = PathBuf::from(&manifest_dir).join("CrSDK_v2.01.00_20260203a_Mac");
    
    // 헤더 경로들
    let header_app = sdk_base.join("RemoteCli/app/CRSDK");
    let lib_path = sdk_base.join("RemoteCli/external/crsdk");

    // 1. C++ 래퍼 컴파일
    cc::Build::new()
        .cpp(true)
        .file("wrapper.cpp")
        .include(&header_app)
        // 만약 다른 경로에 헤더가 더 있다면 .include()를 추가하세요.
        .std("c++17")
        .warnings(false) // SDK 내부 경고 무시
        .compile("sony_shim");

    // 2. 링커 설정
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib=Cr_Core");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());

    // 3. Bindgen 설정
    let bindings = bindgen::Builder::default()
        .header("wrapper.cpp")
        .clang_arg(format!("-I{}", header_app.display()))
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++17")
        .allowlist_function("sdk_init")
        .allowlist_function("sdk_release") // 이 줄이 없으면 Rust에서 호출이 불가능합니다.
        .allowlist_function("get_sdk_version")
        .generate()
        .expect("Unable to generate bindings");

    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = "./bindings.rs";
    bindings
        // .write_to_file(out_path.join("bindings.rs"))
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}