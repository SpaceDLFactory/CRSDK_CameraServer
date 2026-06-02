// src/callback.rs — bridges SDK IDeviceCallback events to std::sync::mpsc
//
// Memory layout:
//   CallbackInner (Box, heap-stable) ← tx + all C fn pointers reference this
//   DeviceCallback owns:
//     - Box<CallbackInner>   (tx lives here; stable address passed as user_data)
//     - cb_ptr: *mut c_void  (opaque C++ RustDeviceCallback object)
//
// Drop safety:
//   Camera::drop calls ffi::deactivate_device_callback(cb_ptr) FIRST,
//   which nulls all fn ptrs in the C++ object → no callbacks after that.
//   DeviceCallback::drop then calls ffi::destroy_callback(cb_ptr) to free
//   the C++ object.  Box<CallbackInner> (and the tx inside it) then drop.

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::mpsc;

use crate::ffi;

/// Events emitted by the SDK via IDeviceCallback.
#[derive(Debug)]
pub enum CameraEvent {
    Connected { version: u32 },
    Disconnected { error: u32 },
    PropertyChanged,
    LvPropertyChanged,
    Warning(u32),
    WarningExt { code: u32, p1: i32, p2: i32, p3: i32 },
    Error(u32),
    /// File transfer complete (capture-to-host).
    /// `filename` is an owned deep copy — the SDK pointer is already gone.
    DownloadComplete { filename: String, kind: u32 },
}

/// Inner state kept on the heap so its address is stable after Box allocation.
struct CallbackInner {
    tx: mpsc::Sender<CameraEvent>,
}

/// Owns the C++ callback object and the Rust mpsc sender.
/// Obtain an `mpsc::Receiver<CameraEvent>` from `DeviceCallback::new()`.
pub struct DeviceCallback {
    _inner: Box<CallbackInner>, // keep alive; C side holds raw pointer into it
    pub(crate) cb_ptr: *mut c_void,
}

// Safety: the C++ callback object is only invoked from SDK threads which are
// thread-safe by the SDK's own contract; mpsc::SyncSender is Send.
unsafe impl Send for DeviceCallback {}

impl DeviceCallback {
    /// Create a new callback object and return (DeviceCallback, Receiver).
    pub fn new() -> (Self, mpsc::Receiver<CameraEvent>) {
        let (tx, rx) = mpsc::channel::<CameraEvent>();

        let mut inner = Box::new(CallbackInner { tx });
        let userdata = &mut *inner as *mut CallbackInner as *mut c_void;

        let cb_ptr = unsafe {
            ffi::create_callback(
                Some(c_on_connected),
                Some(c_on_disconnected),
                Some(c_on_prop_changed),
                Some(c_on_lv_prop_changed),
                Some(c_on_warning),
                Some(c_on_warning_ext),
                Some(c_on_error),
                Some(c_on_complete_download),
                userdata,
            )
        };

        (DeviceCallback { _inner: inner, cb_ptr }, rx)
    }
}

impl Drop for DeviceCallback {
    fn drop(&mut self) {
        // deactivate should already have been called by Camera::drop, but
        // calling it here too is harmless (idempotent null-zeroing).
        unsafe {
            ffi::deactivate_device_callback(self.cb_ptr);
            ffi::destroy_callback(self.cb_ptr);
        }
    }
}

// ── C callback shims ────────────────────────────────────────────────────────
//
// Each shim reconstructs &CallbackInner from userdata and sends via
// an unbounded mpsc channel.  send() on an unbounded channel never blocks,
// so the SDK background thread is never stalled.

unsafe extern "C" fn c_on_connected(ud: *mut c_void, version: u32) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::Connected { version });
}

unsafe extern "C" fn c_on_disconnected(ud: *mut c_void, error: u32) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::Disconnected { error });
}

unsafe extern "C" fn c_on_prop_changed(ud: *mut c_void) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::PropertyChanged);
}

unsafe extern "C" fn c_on_lv_prop_changed(ud: *mut c_void) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::LvPropertyChanged);
}

unsafe extern "C" fn c_on_warning(ud: *mut c_void, code: u32) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::Warning(code));
}

unsafe extern "C" fn c_on_warning_ext(
    ud: *mut c_void, code: u32, p1: i32, p2: i32, p3: i32,
) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::WarningExt { code, p1, p2, p3 });
}

unsafe extern "C" fn c_on_error(ud: *mut c_void, code: u32) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    let _ = inner.tx.send(CameraEvent::Error(code));
}

unsafe extern "C" fn c_on_complete_download(
    ud: *mut c_void,
    filename: *const c_char,
    kind: u32,
) {
    if ud.is_null() { return; }
    let inner = &*(ud as *const CallbackInner);
    // Deep-copy before returning — the SDK string pointer is only valid for
    // the duration of this callback invocation.
    let owned = if filename.is_null() {
        String::new()
    } else {
        CStr::from_ptr(filename).to_string_lossy().into_owned()
    };
    let _ = inner.tx.send(CameraEvent::DownloadComplete { filename: owned, kind });
}
