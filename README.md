# TetherMoon 🌙

![TetherMoon](gallery/sns-wide.png)

*[한국어 README](README.ko.md) · [日本語 README](README.ja.md)*

A Rust FFI wrapper for the **Sony Camera Remote SDK** plus a browser-based
**tethering server** with a single-page web UI. Control exposure, focus, capture,
live view, and long-exposure/timelapse from any browser on your phone or PC.

> ### ⚠️ Target device: Sony A7C (ILCE-7C) only
> This tool was developed and tested with **a single body, the ILCE-7C**, over
> USB on macOS (Apple Silicon). Other bodies are untested. Features the A7C does
> not expose (gyro level, Creative Look, bulb timer, AF-area device properties,
> etc.) remain in the code but do not work on this body. Multi-body support is
> a future goal.

## Quick start — just use it

No building required. Grab the latest **[release](../../releases/latest)**:

1. Download the `.dmg`, open it, and drag **TetherMoon** into Applications.
2. Launch it. First time only: right-click the app → **Open** → **Open**.
3. Connect your A7C by USB and set it to **PC Remote** (camera: MENU → USB →
   *USB Connection Mode* → *PC Remote*). The console opens in your browser.
4. To watch/control from a phone, open the LAN URL shown at the bottom of the
   page (phone must be on the same Wi-Fi).

The rest of this README is for **building from source**.

## Features

- **Live view** — MJPEG stream with focus peaking, RGB histogram, toggleable
  rule-of-thirds grid (rotates with the view), manual rotation
- **Exposure & color** — ISO, shutter, aperture, EV, white balance (+ Kelvin slider),
  metering, drive mode, flash mode, file format, JPEG quality, Picture Profile
- **Focus** — MF Near/Far slider, AF point by live-view click (Y-axis calibrated,
  rotation-aware), AF-area mode (Wide/Zone/Center/Flexible S·M·L/Tracking),
  half-shutter (S1) with focus-indication feedback
- **Capture** — single, burst (press-hold), movie record, cancel
- **Long exposure** — fixed 1″–30″, BULB, and a **software bulb timer** (1–900 s)
- **Timelapse** — software interval shooting (count × interval) with cancel
- **Save** — to PC with custom folder/prefix, capture preview, battery & shots-remaining
- **Multi-body ready** — controls are curated from each body's reported
  capabilities; properties a body does not expose are hidden automatically
- **Multiple viewers** — the live view fans out to any number of browsers
  (phone + desktop at once) from a single camera stream
- **Robust** — auto-reconnect, graceful shutdown (clean camera session release),
  opens your browser automatically on launch

## Screenshots

The single-page **Tether Console** — live view with focus peaking and a rule-of-thirds
grid on the left, all controls on the right.

| Connected | Live view (MF focus pull) |
|---|---|
| ![connected UI](gallery/ui-connected.png) | ![live view](gallery/ui-liveview.png) |

![full UI, disconnected](gallery/ui-disconnected.png)

## Architecture

```
Sony C++ SDK ──► wrapper/wrapper.{h,cpp}  (pure-C shim, SCRSDK namespace bridge)
                     └─► build.rs (cc + bindgen) ─► src/ffi.rs
                            └─► safe Rust lib: session / enumerate / connection /
                                liveview / shutter / control / properties / callback / error
                                   └─► crsdk_server (axum/tokio) + crsdk_server/web/index.html
```

All SDK calls run on `spawn_blocking`; the camera lives behind `Arc<Mutex<…>>`.

## Build

The **Sony SDK is not included** in this repository (see *License* below). Download
it yourself and place it at the project root as `CrSDK_v2.01.00_20260203a_Mac/`.

```bash
# Prerequisites: Rust, LLVM/Clang (brew install llvm)
export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:$(pwd)/CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk/

cargo run -p crsdk_server        # → http://localhost:8080/web/index.html
```

macOS's `ptpcamerad` daemon interferes with USB camera access; the server suppresses
it on startup (this is expected behavior).

## Distribution (binary .app)

The Sony license permits distributing the SDK library **embedded inside your
application**. `scripts/make_app.sh` packages a self-contained macOS app bundle
(`dist/TetherMoon.app`) with the SDK libraries inside `Contents/Frameworks/`:

```bash
./scripts/make_app.sh
```

Pre-built releases are attached to the [Releases](../../releases) page. First launch:
right-click → Open, or `xattr -dr com.apple.quarantine "TetherMoon.app"`.

## 🌙 First shot

Taken with this tool — ILCE-7C + FE 100-400 GM, straight out of camera.

![first moon](gallery/first-moon.jpg)

> © neko.kim.film (김괭필름)

## Support

If this project is useful to you:

[![Support on Ctee](https://img.shields.io/badge/Ctee-sdlfactory-FF5A5F)](https://ctee.kr/place/sdlfactory)

## Contact

Questions, bug reports, or feedback: **spacedlfactory@gmail.com**

## License

The source code in this repository is **MIT licensed** ([LICENSE](LICENSE)).

The **Sony Camera Remote SDK is NOT included** and is **copyright Sony**. You must
download it from [Sony's Developer World](https://www.sony.net/CameraRemoteSDK/) and
agree to its
[License Agreement](https://support.d-imaging.sony.co.jp/app/sdk/licenseagreement/en.html).
This is an independent, unofficial project, not affiliated with or endorsed by Sony.
