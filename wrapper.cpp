#include "CameraRemote_SDK.h"

// 네임스페이스 사용 선언
using namespace SCRSDK;

extern "C" {
    // Rust에서 호출할 초기화 함수
    bool sdk_init(unsigned int log_type) {
        // 헤더 정의: bool Init(CrInt32u logtype = 0);
        return Init(log_type);
    }

    // 헤더 정의: CrInt32u GetSDKVersion();
    unsigned int get_sdk_version() {
        return GetSDKVersion();
    }

    // 헤더 정의: bool Release();
    bool sdk_release() {
        return Release();
    }
}