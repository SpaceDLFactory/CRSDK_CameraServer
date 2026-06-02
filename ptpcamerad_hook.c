/*
 * ptpcamerad_hook.c
 *
 * macOS에서 ptpcamerad가 USB 인터페이스를 선점하는 문제를 해결하는 훅.
 * DYLD_INSERT_LIBRARIES로 삽입되어 libusb_claim_interface 직전에
 * ptpcamerad를 종료 후 즉시 실제 claim을 호출.
 *
 * 빌드:
 *   clang -shared -fPIC -o ptpcamerad_hook.dylib ptpcamerad_hook.c -ldl
 *
 * 사용:
 *   DYLD_INSERT_LIBRARIES=./ptpcamerad_hook.dylib \
 *   DYLD_FORCE_FLAT_NAMESPACE=1 \
 *   ./target/debug/crsdk_rust_wrapper
 */
#include <dlfcn.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int libusb_claim_interface(void *dev, int interface_number) {
    /* ptpcamerad 종료 — USB 인터페이스 exclusive hold 해제 */
    system("kill $(pgrep ptpcamerad) 2>/dev/null");

    typedef int (*real_fn)(void *, int);
    real_fn fn = (real_fn)dlsym(RTLD_NEXT, "libusb_claim_interface");
    int ret = fn(dev, interface_number);
    fprintf(stderr, "[ptpcamerad_hook] claim_interface(if=%d) = %d\n",
            interface_number, ret);
    return ret;
}

/* detach_kernel_driver는 macOS에서 항상 실패하므로 0(성공)으로 덮어씀 */
int libusb_detach_kernel_driver(void *dev, int interface_number) {
    system("kill $(pgrep ptpcamerad) 2>/dev/null");
    return 0;
}
