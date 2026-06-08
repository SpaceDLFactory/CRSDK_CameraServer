use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // 타깃 OS (build.rs는 호스트에서 돌지만 cargo가 타깃을 env로 준다)
    let is_windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");

    // SDK 헤더/라이브러리 경로 — OS별 폴더명·하위구조가 다름.
    //   macOS  : CrSDK_..._Mac/RemoteCli/{app/CRSDK, external/crsdk}
    //   Windows: CrSDK_Win/{app/CRSDK, external/crsdk}  (RemoteCli.zip 풀면 RemoteCli 폴더 없이 바로)
    let (sdk_include, sdk_lib) = if is_windows {
        let root = manifest.join("CrSDK_Win");
        (root.join("app/CRSDK"), root.join("external/crsdk"))
    } else {
        let root = manifest.join("CrSDK_v2.01.00_20260203a_Mac/RemoteCli");
        (root.join("app/CRSDK"), root.join("external/crsdk"))
    };

    // Wrapper sources
    let wrapper_dir = manifest.join("wrapper");
    let wrapper_h   = wrapper_dir.join("wrapper.h");
    let wrapper_cpp = wrapper_dir.join("wrapper.cpp");

    // Sanity checks
    assert!(sdk_include.exists(), "SDK headers not found: {sdk_include:?}");
    assert!(wrapper_h.exists(),   "wrapper.h not found");
    assert!(wrapper_cpp.exists(), "wrapper.cpp not found");

    // Compile wrapper.cpp → libwrapper.a
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").include(&sdk_include).file(&wrapper_cpp);
    if is_windows {
        build.flag_if_supported("/GR-"); // MSVC: RTTI off (예외 끄기는 SDK 헤더 의존 — 추후 실측)
    } else {
        build.flag("-fno-rtti").flag("-fno-exceptions"); // Clang/GCC
    }
    build.compile("wrapper");

    // Link the Sony SDK library (macOS: libCr_Core.dylib / Windows: Cr_Core.lib→Cr_Core.dll)
    println!("cargo:rustc-link-search=native={}", sdk_lib.display());
    println!("cargo:rustc-link-lib=dylib=Cr_Core");

    // rpath는 ELF/Mach-O 전용. macOS는 dylib 디렉터리를 rpath로 박아 DYLD_LIBRARY_PATH 없이
    // 찾게 한다. Windows엔 rpath 개념이 없어(DLL은 exe 옆/PATH) 건너뛴다.
    if !is_windows {
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg={}", sdk_lib.display());
    }

    // Re-run if wrapper sources change
    println!("cargo:rerun-if-changed=wrapper/wrapper.h");
    println!("cargo:rerun-if-changed=wrapper/wrapper.cpp");

    // ── CrAdapter plugin symlink ─────────────────────────────────────────────
    //
    // libCr_Core.dylib looks up USB transport plugins via NSBundle:
    //   NSBundle.mainBundle.bundlePath + "Contents/Frameworks/CrAdapter"
    //
    // For a CLI binary at target/debug/crsdk_example, mainBundle.bundlePath is
    // target/debug/, so the SDK expects:
    //   target/debug/Contents/Frameworks/CrAdapter/libCr_PTP_USB.dylib
    //
    // We create a symlink from that path to the actual CrAdapter directory.
    // OUT_DIR is target/debug/build/crsdk-<hash>/out  → go up 3 levels = target/debug/
    // (macOS/unix 전용 — std::os::unix::symlink. Windows는 NSBundle이 없고 플러그인 lookup
    //  규칙이 달라 별도 처리 필요.)
    #[cfg(unix)]
    {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        // OUT_DIR: <profile>/build/<crate>-<hash>/out → ancestor(3) = <profile dir>
        if let Some(binary_dir) = out_dir.ancestors().nth(3) {
            let contents_dir = binary_dir.join("Contents/Frameworks");
            std::fs::create_dir_all(&contents_dir)
                .expect("failed to create Contents/Frameworks/");
            let adapter_link = contents_dir.join("CrAdapter");
            if !adapter_link.exists() {
                std::os::unix::fs::symlink(sdk_lib.join("CrAdapter"), &adapter_link)
                    .expect("failed to symlink CrAdapter");
                println!("cargo:warning=Created CrAdapter symlink: {adapter_link:?}");
            }
        }
    }
    // TODO(windows): CrAdapter DLL(Cr_PTP_USB.dll 등)을 exe 옆에 복사. SDK의 Windows 플러그인
    // lookup 규칙(exe 기준 CrAdapter/ 인지 등)을 실측 후 확정.
    #[cfg(windows)]
    {
        let _ = &sdk_lib; // (Windows 세션에서 구현)
    }

    // Generate Rust FFI bindings from wrapper.h
    let bindings = bindgen::Builder::default()
        .header(wrapper_h.to_str().unwrap())
        // wrapper.h is pure C — suppress C++ noise
        .clang_arg("-x")
        .clang_arg("c")
        // Keep only the symbols we declared
        .allowlist_function("sdk_init")
        .allowlist_function("sdk_release")
        .allowlist_function("get_sdk_version")
        .allowlist_function("get_sdk_serial")
        .allowlist_function("enum_cameras")
        .allowlist_function("enum_get_count")
        .allowlist_function("enum_get_camera_ptr")
        .allowlist_function("enum_release")
        .allowlist_function("camera_get_name_ptr")
        .allowlist_function("camera_get_name_size")
        .allowlist_function("camera_get_model_ptr")
        .allowlist_function("camera_get_model_size")
        .allowlist_function("camera_get_connection_type_name_ptr")
        .allowlist_function("camera_get_usb_pid")
        .allowlist_function("camera_get_connection_status")
        .allowlist_function("camera_get_ssh_support")
        .allowlist_function("camera_get_fingerprint")
        .allowlist_function("create_callback")
        .allowlist_function("deactivate_device_callback")
        .allowlist_function("destroy_callback")
        .allowlist_function("camera_connect")
        .allowlist_function("camera_disconnect")
        .allowlist_function("camera_release_device")
        .allowlist_function("camera_send_command")
        .allowlist_function("liveview_get_buffer_size")
        .allowlist_function("liveview_alloc_block")
        .allowlist_function("liveview_fetch")
        .allowlist_function("liveview_free_block")
        .allowlist_function("liveview_get_level")
        .allowlist_type("CrLevelSimple")
        .allowlist_function("liveview_get_af_frame")
        .allowlist_type("CrAfFrameSimple")
        .allowlist_function("get_device_properties")
        .allowlist_function("release_device_properties_simple")
        .allowlist_function("set_device_property")
        .allowlist_function("get_device_setting")
        .allowlist_function("set_device_setting")
        .allowlist_function("set_save_info")
        .allowlist_function("execute_control_code_value")
        .allowlist_function("get_control_code_info")
        .allowlist_type("CrControlInfoSimple")
        .allowlist_function("get_property_string")
        .allowlist_type("CrPropertySimple")
        .generate()
        .expect("bindgen failed to generate bindings");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");
}
