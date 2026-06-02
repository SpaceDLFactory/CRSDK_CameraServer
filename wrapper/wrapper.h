/* wrapper.h — Pure C interface for Sony CrSDK
 *
 * Callback lifecycle (Drop-safe pattern):
 *   create_callback(...)             → allocates C++ IDeviceCallback object
 *   deactivate_device_callback(ptr)  → nulls all fn ptrs (call BEFORE disconnect)
 *   destroy_callback(ptr)            → frees C++ object (call AFTER release_device)
 *
 * Typical Drop order in Rust:
 *   1. deactivate_device_callback  ← silences lingering SDK background callbacks
 *   2. camera_disconnect
 *   3. camera_release_device
 *   4. destroy_callback            ← free C++ heap object
 */

#ifndef WRAPPER_H
#define WRAPPER_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------
 * Flat property struct — avoids C++/C layout mismatch with
 * CrDeviceProperty (which uses virtual methods and private pimpls).
 * ------------------------------------------------------------------ */
#define CR_PROPERTY_MAX_ALLOWED 128

typedef struct {
    uint32_t code;          /* CrDevicePropertyCode */
    uint32_t value_type;    /* CrDataType */
    uint64_t current_value; /* CrInt64u */
    uint8_t  is_editable;   /* 1 if the property can be written right now */
    uint8_t  _pad[3];
    uint32_t allowed_count; /* number of valid entries in allowed_values (≤ CR_PROPERTY_MAX_ALLOWED) */
    uint64_t allowed_values[CR_PROPERTY_MAX_ALLOWED]; /* allowed/settable values */
} CrPropertySimple;

/* ==================================================================
 * SDK lifecycle
 * ================================================================== */

/* Returns 0 on success, non-zero on failure. */
int32_t  sdk_init(int32_t log_type);
int32_t  sdk_release(void);
uint32_t get_sdk_version(void);
uint32_t get_sdk_serial(void);

/* ==================================================================
 * Enumeration — discovers cameras connected to the PC
 * ================================================================== */

/* Enumerate cameras. timeout_sec: seconds to scan (SDK default = 3).
 * *out_handle receives ICrEnumCameraObjectInfo* (opaque).
 * Returns CrError (0 = success). */
int32_t     enum_cameras(void** out_handle, uint8_t timeout_sec);

/* Number of discovered cameras. */
uint32_t    enum_get_count(const void* handle);

/* Raw ICrCameraObjectInfo* for index n.
 * OWNED by the enum handle — valid only while handle is alive. */
const void* enum_get_camera_ptr(const void* handle, uint32_t index);

/* Release enum handle. */
void        enum_release(void* handle);

/* ------------------------------------------------------------------
 * Camera info accessors (operate on const void* from enum_get_camera_ptr)
 * ------------------------------------------------------------------ */
const char* camera_get_name_ptr(const void* cam_ptr);
uint32_t    camera_get_name_size(const void* cam_ptr);
const char* camera_get_model_ptr(const void* cam_ptr);
uint32_t    camera_get_model_size(const void* cam_ptr);
uint16_t    camera_get_usb_pid(const void* cam_ptr);
uint32_t    camera_get_connection_status(const void* cam_ptr);
/* 0 = SSH off (USB camera), 1 = SSH on (WiFi camera).
 * Maps to CrSSHsupportValue. */
uint32_t    camera_get_ssh_support(const void* cam_ptr);
/* Fetch SSH host-key fingerprint from the camera (network round-trip).
 * buf_size: size of caller buffer (1024 recommended).
 * Returns actual fingerprint length on success, 0 on failure/no-SSH. */
uint32_t    camera_get_fingerprint(const void* cam_ptr,
                                    char* buf, uint32_t buf_size);

/* ==================================================================
 * Callback — bridges SDK IDeviceCallback events to Rust channels
 * ================================================================== */

/* Allocate a C++ IDeviceCallback object wired to the given function
 * pointers and user_data.  All fn pointers are nullable.
 * Returns opaque void* (must be freed with destroy_callback).
 *
 * fn_complete_download: fired when a file transfer (capture-to-host) finishes.
 *   filename — null-terminated UTF-8 path on the host; pointer is valid ONLY
 *              for the duration of the callback.  Deep-copy immediately.
 *   kind     — SDK-defined transfer type (CrInt32u). */
void* create_callback(
    void (*fn_connected)          (void* ud, uint32_t version),
    void (*fn_disconnected)       (void* ud, uint32_t error),
    void (*fn_prop_changed)       (void* ud),
    void (*fn_lv_prop_changed)    (void* ud),
    void (*fn_warning)            (void* ud, uint32_t code),
    void (*fn_warning_ext)        (void* ud, uint32_t code,
                                   int32_t p1, int32_t p2, int32_t p3),
    void (*fn_error)              (void* ud, uint32_t code),
    void (*fn_complete_download)  (void* ud, const char* filename, uint32_t kind),
    void* user_data
);

/* Null all function pointers in the callback object.
 * Call this BEFORE camera_disconnect to prevent use-after-free:
 * any in-flight SDK callbacks become no-ops after this call. */
void deactivate_device_callback(void* cb_ptr);

/* Free the C++ callback object.
 * Safe only after camera_release_device has returned. */
void destroy_callback(void* cb_ptr);

/* ==================================================================
 * Connection
 * ================================================================== */

/* Async connect request.  On success the SDK eventually fires
 * fn_connected via cb_ptr.
 *
 * open_mode           : 0 = CrSdkControlMode_Remote (normal PC remote)
 * reconnect           : 1 = CrReconnecting_ON
 * user_id             : "admin" for both USB and WiFi
 * password            : "" for USB; SSH password for WiFi
 * fingerprint         : NULL or "" for USB; SSH host-key bytes for WiFi
 * fingerprint_size    : 0 for USB; fingerprint byte count for WiFi
 * pairing_display_name: NULL for USB; UTF-8 name shown on camera LCD
 *                       during first-time WiFi pairing (e.g. "CrSDK-Rust").
 *                       Ignored if already paired.
 *
 * Returns CrError (0 = request accepted, callback pending). */
int32_t camera_connect(
    const void* cam_ptr,
    void*       cb_ptr,
    int64_t*    out_handle,
    int32_t     open_mode,
    int32_t     reconnect,
    const char* user_id,
    const char* password,
    const char* fingerprint,
    uint32_t    fingerprint_size,
    const char* pairing_display_name
);

int32_t camera_disconnect   (int64_t handle);
int32_t camera_release_device(int64_t handle);

/* ==================================================================
 * Commands (shutter, video, etc.)
 * ================================================================== */

/* SendCommand wrapper.  See CrCommandData.h for IDs and params. */
int32_t camera_send_command(int64_t handle,
                             uint32_t command_id,
                             uint16_t command_param);

/* ==================================================================
 * LiveView — three-step acquire loop
 *
 * Step 1: liveview_get_buffer_size  → *out_buf_size
 * Step 2: liveview_alloc_block(size, &buf) → block ptr
 * Step 3: liveview_fetch(handle, block, ...)  (repeat per frame)
 * Step 4: liveview_free_block(block, buf)
 * ================================================================== */

int32_t liveview_get_buffer_size(int64_t handle, uint32_t* out_buf_size);

/* Allocates CrImageDataBlock + pixel buffer.
 * *out_buf receives the raw pixel buffer pointer (needed for free). */
void* liveview_alloc_block(uint32_t buf_size, uint8_t** out_buf);

/* Fetch one frame into block.  *out_image_data points inside block
 * (zero-copy) and is valid until next fetch or free_block. */
int32_t liveview_fetch(int64_t handle, void* block,
                        uint32_t* out_image_size,
                        const uint8_t** out_image_data);

void liveview_free_block(void* block, uint8_t* buf);

/* LiveView Level (gyro): 카메라 중력센서 롤/피치.
 * state: CrLevelState (1=Off, 2=On). x/y/z 는 state==2 일 때만 유효 (x=롤). */
typedef struct {
    int32_t state;
    int32_t x;
    int32_t y;
    int32_t z;
} CrLevelSimple;

int32_t liveview_get_level(int64_t handle, CrLevelSimple* out);

/* LiveView AF 프레임 실위치 (CrFocusFrameInfo). 위치는 분수: x=x_num/x_deno, y=y_num/y_deno.
 * valid=1 이면 프레임을 받음(첫 프레임만). 명령 좌표 vs 실제 박스 위치 보정용. */
typedef struct {
    int32_t  valid;
    uint32_t x_num, x_deno, y_num, y_deno;
    uint32_t width, height;
} CrAfFrameSimple;

int32_t liveview_get_af_frame(int64_t handle, CrAfFrameSimple* out);

/* ==================================================================
 * Device properties (simplified flat API)
 * ================================================================== */

/* Fills *out_props with a wrapper-allocated CrPropertySimple array.
 * Caller must free with release_device_properties_simple. */
int32_t get_device_properties(int64_t handle,
                               CrPropertySimple** out_props,
                               uint32_t* out_count);

int32_t release_device_properties_simple(CrPropertySimple* props);

int32_t set_device_property(int64_t handle,
                             const CrPropertySimple* prop);

/* ==================================================================
 * Device settings / save path
 * ================================================================== */

int32_t get_device_setting(int64_t handle, uint32_t key, uint32_t* value);
int32_t set_device_setting(int64_t handle, uint32_t key, uint32_t value);
int32_t set_save_info     (int64_t handle,
                            const char* path,
                            const char* prefix,
                            int32_t no);

/* ExecuteControlCodeValue — ControlCode 기반 즉시 작용 명령
 * (NearFar=0xD2D1 MF 포커스 이동, AFAreaPosition=0xD2DC, ZoomOperation=0xD2DD 등). */
int32_t execute_control_code_value(int64_t handle, uint32_t code, uint64_t value);

/* GetSelectControlCode → 단일 ControlCode 의 허용 값/범위 조회.
 * value_type 의 RangeBit(0x4000)가 켜져 있으면 values = [min, step, max] (3 원소).
 * ArrayBit(0x2000)면 values = 허용 값 목록. SignBit(0x1000)면 부호 있음. */
#define CR_CONTROL_MAX_VALUES 32
typedef struct {
    uint32_t value_type;                          /* CrDataType */
    uint32_t count;                               /* values[] 유효 원소 수 */
    uint64_t values[CR_CONTROL_MAX_VALUES];       /* raw (부호 해석은 caller) */
} CrControlInfoSimple;

int32_t get_control_code_info(int64_t handle, uint32_t code, CrControlInfoSimple* out);

/* STR 타입 속성의 현재 문자열을 UTF-8로 반환. 반환값=쓴 바이트 수(널 제외),
 * 0=속성 없음/문자열 아님. Sony 의 GetCurrentStr 는 CrInt16u*(UTF-16).
 * 단순화를 위해 비-ASCII 코드포인트는 '?' 로 치환 (렌즈명 등은 보통 ASCII). */
int32_t get_property_string(int64_t handle, uint32_t code, char* buf, uint32_t buf_size);

#ifdef __cplusplus
}
#endif

#endif /* WRAPPER_H */
