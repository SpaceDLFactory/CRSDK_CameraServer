use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // SDK headers live here (CameraRemote_SDK.h, IDeviceCallback.h, ...)
    let sdk_include = manifest
        .join("CrSDK_v2.01.00_20260203a_Mac/RemoteCli/app/CRSDK");

    // SDK dylib lives here (libCr_Core.dylib, ...)
    let sdk_lib = manifest
        .join("CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk");

    // Wrapper sources
    let wrapper_dir = manifest.join("wrapper");
    let wrapper_h   = wrapper_dir.join("wrapper.h");
    let wrapper_cpp = wrapper_dir.join("wrapper.cpp");

    // Sanity checks
    assert!(sdk_include.exists(), "SDK headers not found: {sdk_include:?}");
    assert!(wrapper_h.exists(),   "wrapper.h not found");
    assert!(wrapper_cpp.exists(), "wrapper.cpp not found");

    // Compile wrapper.cpp → libwrapper.a
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-fno-rtti")
        .flag("-fno-exceptions")
        .include(&sdk_include)
        .file(&wrapper_cpp)
        .compile("wrapper");

    // Link the Sony SDK dynamic library
    println!("cargo:rustc-link-search=native={}", sdk_lib.display());
    println!("cargo:rustc-link-lib=dylib=Cr_Core");

    // Embed the dylib directory as an rpath so dyld can find libCr_Core.dylib
    // at runtime without requiring DYLD_LIBRARY_PATH to be set manually.
    // The path is absolute and machine-specific, which is fine for a local
    // development binary.
    println!("cargo:rustc-link-arg=-rpath");
    println!("cargo:rustc-link-arg={}", sdk_lib.display());

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
