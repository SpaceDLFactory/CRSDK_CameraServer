#!/usr/bin/env bash
# test.sh — CrSDK Rust Wrapper 하드웨어 테스트 스크립트
#
# Usage:
#   ./test.sh          # Phase 3 (연결만)
#   ./test.sh 4        # Phase 4 (LiveView, 30초 자동 종료)
#   ./test.sh 5        # Phase 5 (셔터, MF 모드 필수)
#   ./test.sh all      # Phase 3→4→5 전체
#   ./test.sh check    # 빌드만 (카메라 불필요)

set -euo pipefail
cd "$(dirname "$0")"

PHASE="${1:-3}"
SDK_LIB="CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk"

# ── 색상 ─────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
fail()  { echo -e "${RED}[FAIL]${NC} $*"; }

# ── 사전 검사 ────────────────────────────────────────────────────────────────

info "SDK dylib: ${SDK_LIB}"
if [ ! -d "$SDK_LIB" ]; then
    fail "SDK library directory not found: ${SDK_LIB}"
    exit 1
fi

# build-only 모드
if [ "$PHASE" = "check" ]; then
    info "Running cargo check + clippy..."
    cargo check 2>&1 | tail -1
    cargo clippy 2>&1 | grep "^error" || ok "clippy clean"
    exit 0
fi

# ── 빌드 ─────────────────────────────────────────────────────────────────────

info "Building (debug)..."
if ! cargo build 2>&1 | tail -3; then
    fail "Build failed"
    exit 1
fi
ok "Build complete"

# ── 환경 설정 ────────────────────────────────────────────────────────────────

export DYLD_LIBRARY_PATH="${SDK_LIB}:${DYLD_LIBRARY_PATH:-}"

# ── 카메라 연결 확인 ─────────────────────────────────────────────────────────

info "Checking USB devices for Sony camera..."
if system_profiler SPUSBDataType 2>/dev/null | grep -qi "sony\|ILCE"; then
    ok "Sony camera detected on USB"
else
    warn "No Sony camera found on USB. Make sure:"
    warn "  1. Camera is ON"
    warn "  2. USB cable connected"
    warn "  3. Camera USB mode = [PC Remote]"
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# ── 간섭 프로세스 사전 정리 ──────────────────────────────────────────────────

info "Killing interfering processes..."
pkill -KILL ptpcamerad 2>/dev/null || true
pkill -KILL "Android File Transfer Agent" 2>/dev/null || true
launchctl stop com.apple.ptpcamerad 2>/dev/null || true
sleep 0.5

# ── 실행 ─────────────────────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Running: crsdk_example --phase ${PHASE}"
echo "  Ctrl+C to stop LiveView loop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cargo run --bin crsdk_example -- --phase "$PHASE"
EXIT_CODE=$?

# ── 결과 ─────────────────────────────────────────────────────────────────────

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    ok "Test completed successfully (exit=$EXIT_CODE)"

    # LiveView JPEG 파일 요약
    JPEG_COUNT=$(ls liveview_*.jpg 2>/dev/null | wc -l | tr -d ' ')
    if [ "$JPEG_COUNT" -gt 0 ]; then
        TOTAL_SIZE=$(du -sh liveview_*.jpg 2>/dev/null | tail -1 | cut -f1)
        info "LiveView frames saved: ${JPEG_COUNT} files"
        info "To view: open liveview_000001.jpg"
        info "To clean: rm liveview_*.jpg"
    fi
else
    fail "Test failed (exit=$EXIT_CODE)"
fi

exit $EXIT_CODE
