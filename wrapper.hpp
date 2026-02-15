// wrapper.hpp
#ifndef WRAPPER_HPP
#define WRAPPER_HPP

// 1. 메인 SDK 헤더 (가장 중요)
#include "CameraRemote_SDK.h"

// 2. 기본 타입 및 에러 코드 정의
#include "CrTypes.h"
#include "CrDefines.h"
#include "CrError.h"

// 3. 카메라 제어 및 프로퍼티 관련
#include "CrCommandData.h"
#include "CrControlCode.h"
#include "CrDeviceProperty.h"

// 4. 인터페이스 정의
#include "ICrCameraObjectInfo.h"
#include "IDeviceCallback.h"

#endif