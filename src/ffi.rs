// Raw FFI 바인딩. build.rs의 bindgen이 OUT_DIR/bindings.rs를 자동 생성함.
// 이 파일은 외부에 직접 노출하지 않음 — pub(crate) 범위.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
