#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

fn main() {
    println!("--- SDL Factory: Sony Camera Remote SDK ---");

    unsafe {
        let v = get_sdk_version();
        println!("SDK Version: {}", v);

        if sdk_init(0) {
            println!("✅ Sony SDK 초기화 성공!");
            
            // 여기에 카메라 탐색 등의 로직이 들어갑니다.
            
            if sdk_release() {
                println!("Released!")
            }
        } else {
            println!("❌ SDK 초기화 실패");
        }
    }
}