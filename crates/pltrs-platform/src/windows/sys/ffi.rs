//! The Win32 declarations the backend uses: functions of `user32`,
//! `gdi32` and `kernel32`, linked with `raw-dylib` (no import libraries
//! needed), and the structures and constants they take.
//!
//! Handles (`HWND`, `HDC`, `HICON`, ...) are pointer-sized integers here.
//! Names and layouts follow the Windows SDK headers; only 64-bit Windows
//! is supported, where `WPARAM`/`LPARAM` are 64 bits wide.

use std::ffi::c_void;

/// `HWND`, `HDC`, `HINSTANCE`, `HICON`, `HCURSOR`, `HBITMAP`, `HGLOBAL`,
/// `HMONITOR`, `HKL`, `HRGN`, `DPI_AWARENESS_CONTEXT`: opaque handles.
pub(super) type Handle = isize;

/// A window procedure (`WNDPROC`).
pub(super) type WindowProcedure = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

/// `POINT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Point {
    pub(super) x: i32,
    pub(super) y: i32,
}

/// `RECT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Rect {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

/// `WNDCLASSEXW`.
#[repr(C)]
pub(super) struct WindowClassEx {
    pub(super) size: u32,
    pub(super) style: u32,
    pub(super) procedure: Option<WindowProcedure>,
    pub(super) class_extra: i32,
    pub(super) window_extra: i32,
    pub(super) instance: Handle,
    pub(super) icon: Handle,
    pub(super) cursor: Handle,
    pub(super) background: Handle,
    pub(super) menu_name: *const u16,
    pub(super) class_name: *const u16,
    pub(super) small_icon: Handle,
}

/// `MSG`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Msg {
    pub(super) window: Handle,
    pub(super) message: u32,
    pub(super) wparam: usize,
    pub(super) lparam: isize,
    pub(super) time: u32,
    pub(super) point: Point,
    pub(super) private: u32,
}

/// `PAINTSTRUCT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PaintStruct {
    pub(super) dc: Handle,
    pub(super) erase: i32,
    pub(super) paint: Rect,
    pub(super) restore: i32,
    pub(super) incremental_update: i32,
    pub(super) reserved: [u8; 32],
}

/// `MINMAXINFO`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MinMaxInfo {
    pub(super) reserved: Point,
    pub(super) max_size: Point,
    pub(super) max_position: Point,
    pub(super) min_track_size: Point,
    pub(super) max_track_size: Point,
}

/// `TRACKMOUSEEVENT`.
#[repr(C)]
pub(super) struct TrackMouseEvent {
    pub(super) size: u32,
    pub(super) flags: u32,
    pub(super) window: Handle,
    pub(super) hover_time: u32,
}

/// `BITMAPINFOHEADER`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BitmapInfoHeader {
    pub(super) size: u32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) planes: u16,
    pub(super) bit_count: u16,
    pub(super) compression: u32,
    pub(super) size_image: u32,
    pub(super) x_pels_per_meter: i32,
    pub(super) y_pels_per_meter: i32,
    pub(super) colors_used: u32,
    pub(super) colors_important: u32,
}

/// `BITMAPINFO`, with its (unused) one-entry colour table.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BitmapInfo {
    pub(super) header: BitmapInfoHeader,
    pub(super) colors: [u32; 1],
}

/// `WINDOWPLACEMENT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowPlacement {
    pub(super) length: u32,
    pub(super) flags: u32,
    pub(super) show: u32,
    pub(super) min_position: Point,
    pub(super) max_position: Point,
    pub(super) normal_position: Rect,
}

/// `MONITORINFO`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MonitorInfo {
    pub(super) size: u32,
    pub(super) monitor: Rect,
    pub(super) work: Rect,
    pub(super) flags: u32,
}

/// `ICONINFO`.
#[repr(C)]
pub(super) struct IconInfo {
    pub(super) icon: i32,
    pub(super) x_hotspot: u32,
    pub(super) y_hotspot: u32,
    pub(super) mask: Handle,
    pub(super) color: Handle,
}

/// `WS_OVERLAPPEDWINDOW`: a title bar, a system menu, sizing borders and
/// minimize and maximize buttons.
pub(super) const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
/// `WS_POPUP`: no title bar nor borders.
pub(super) const WS_POPUP: u32 = 0x8000_0000;
/// `CW_USEDEFAULT`: the system picks the position.
pub(super) const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
/// `HWND_MESSAGE`: the parent of message-only windows.
pub(super) const HWND_MESSAGE: Handle = -3;
/// `GWL_STYLE`.
pub(super) const GWL_STYLE: i32 = -16;
/// `SW_SHOWNORMAL`.
pub(super) const SW_SHOWNORMAL: i32 = 1;
/// `SW_RESTORE`.
pub(super) const SW_RESTORE: i32 = 9;
/// `SWP_NOSIZE`.
pub(super) const SWP_NOSIZE: u32 = 0x0001;
/// `SWP_NOMOVE`.
pub(super) const SWP_NOMOVE: u32 = 0x0002;
/// `SWP_NOZORDER`.
pub(super) const SWP_NOZORDER: u32 = 0x0004;
/// `SWP_NOACTIVATE`.
pub(super) const SWP_NOACTIVATE: u32 = 0x0010;
/// `SWP_FRAMECHANGED`.
pub(super) const SWP_FRAMECHANGED: u32 = 0x0020;
/// `SWP_NOOWNERZORDER`.
pub(super) const SWP_NOOWNERZORDER: u32 = 0x0200;
/// `MONITOR_DEFAULTTONEAREST`.
pub(super) const MONITOR_DEFAULTTONEAREST: u32 = 2;
/// `PM_NOREMOVE`.
pub(super) const PM_NOREMOVE: u32 = 0;
/// `PM_REMOVE`.
pub(super) const PM_REMOVE: u32 = 1;
/// `QS_ALLINPUT`: every kind of message wakes the wait.
pub(super) const QS_ALLINPUT: u32 = 0x04FF;
/// `MWMO_INPUTAVAILABLE`: return at once if messages are queued, seen or
/// not.
pub(super) const MWMO_INPUTAVAILABLE: u32 = 0x0004;
/// `WAIT_FAILED`.
pub(super) const WAIT_FAILED: u32 = u32::MAX;
/// `TME_LEAVE`.
pub(super) const TME_LEAVE: u32 = 0x0002;
/// `WM_SETICON`.
pub(super) const WM_SETICON: u32 = 0x0080;
/// `ICON_SMALL`.
pub(super) const ICON_SMALL: usize = 0;
/// `ICON_BIG`.
pub(super) const ICON_BIG: usize = 1;
/// `BI_RGB`: uncompressed pixels.
pub(super) const BI_RGB: u32 = 0;
/// `DIB_RGB_COLORS`.
pub(super) const DIB_RGB_COLORS: u32 = 0;
/// `RDW_INVALIDATE`: `RedrawWindow` adds the rectangle to the update
/// region.
pub(super) const RDW_INVALIDATE: u32 = 0x0001;
/// `RDW_UPDATENOW`: `RedrawWindow` sends `WM_PAINT` before returning.
pub(super) const RDW_UPDATENOW: u32 = 0x0100;
/// `SRCCOPY`: the raster operation that copies the source.
pub(super) const SRCCOPY: u32 = 0x00CC_0020;
/// `CF_UNICODETEXT`.
pub(super) const CF_UNICODETEXT: u32 = 13;
/// `GMEM_MOVEABLE`.
pub(super) const GMEM_MOVEABLE: u32 = 0x0002;
/// `GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT`.
pub(super) const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x0002;
/// `GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS`.
pub(super) const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0004;
/// `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` (Windows 10 1703).
pub(super) const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: Handle = -4;
/// `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE` (Windows 8.1).
pub(super) const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE: Handle = -3;
/// `ToUnicodeEx`'s flag that leaves the keyboard state (dead keys)
/// unchanged (Windows 10 1607).
pub(super) const TO_UNICODE_KEEP_STATE: u32 = 0x0004;

#[link(name = "user32", kind = "raw-dylib")]
unsafe extern "system" {
    pub(super) fn RegisterClassExW(class: *const WindowClassEx) -> u16;
    pub(super) fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *mut c_void,
    ) -> Handle;
    pub(super) fn DestroyWindow(window: Handle) -> i32;
    pub(super) fn DefWindowProcW(
        window: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
    pub(super) fn ShowWindow(window: Handle, command: i32) -> i32;
    pub(super) fn IsIconic(window: Handle) -> i32;
    pub(super) fn SetForegroundWindow(window: Handle) -> i32;
    pub(super) fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    pub(super) fn SetWindowPos(
        window: Handle,
        insert_after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    pub(super) fn GetWindowLongW(window: Handle, index: i32) -> i32;
    pub(super) fn SetWindowLongW(window: Handle, index: i32, value: i32) -> i32;
    pub(super) fn GetWindowPlacement(window: Handle, placement: *mut WindowPlacement) -> i32;
    pub(super) fn SetWindowPlacement(window: Handle, placement: *const WindowPlacement) -> i32;
    pub(super) fn MonitorFromWindow(window: Handle, flags: u32) -> Handle;
    pub(super) fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
    pub(super) fn AdjustWindowRectExForDpi(
        rect: *mut Rect,
        style: u32,
        menu: i32,
        ex_style: u32,
        dpi: u32,
    ) -> i32;
    pub(super) fn GetDpiForWindow(window: Handle) -> u32;
    pub(super) fn SetThreadDpiAwarenessContext(context: Handle) -> Handle;
    pub(super) fn SendMessageW(window: Handle, message: u32, wparam: usize, lparam: isize)
    -> isize;
    pub(super) fn PostMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
    pub(super) fn PeekMessageW(
        msg: *mut Msg,
        window: Handle,
        filter_min: u32,
        filter_max: u32,
        remove: u32,
    ) -> i32;
    pub(super) fn TranslateMessage(msg: *const Msg) -> i32;
    pub(super) fn DispatchMessageW(msg: *const Msg) -> isize;
    pub(super) fn MsgWaitForMultipleObjectsEx(
        count: u32,
        handles: *const Handle,
        milliseconds: u32,
        wake_mask: u32,
        flags: u32,
    ) -> u32;
    pub(super) fn GetMessageTime() -> i32;
    pub(super) fn GetDoubleClickTime() -> u32;
    pub(super) fn GetKeyState(virtual_key: i32) -> i16;
    pub(super) fn GetKeyboardState(state: *mut u8) -> i32;
    pub(super) fn GetKeyboardLayout(thread: u32) -> Handle;
    pub(super) fn ToUnicodeEx(
        virtual_key: u32,
        scan_code: u32,
        key_state: *const u8,
        buffer: *mut u16,
        buffer_len: i32,
        flags: u32,
        layout: Handle,
    ) -> i32;
    pub(super) fn TrackMouseEvent(track: *mut TrackMouseEvent) -> i32;
    pub(super) fn SetCapture(window: Handle) -> Handle;
    pub(super) fn ReleaseCapture() -> i32;
    pub(super) fn ScreenToClient(window: Handle, point: *mut Point) -> i32;
    pub(super) fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    pub(super) fn SetCursor(cursor: Handle) -> Handle;
    pub(super) fn CreateIconIndirect(info: *const IconInfo) -> Handle;
    pub(super) fn DestroyIcon(icon: Handle) -> i32;
    pub(super) fn BeginPaint(window: Handle, paint: *mut PaintStruct) -> Handle;
    pub(super) fn EndPaint(window: Handle, paint: *const PaintStruct) -> i32;
    pub(super) fn RedrawWindow(
        window: Handle,
        update: *const Rect,
        region: Handle,
        flags: u32,
    ) -> i32;
    pub(super) fn OpenClipboard(owner: Handle) -> i32;
    pub(super) fn CloseClipboard() -> i32;
    pub(super) fn EmptyClipboard() -> i32;
    pub(super) fn SetClipboardData(format: u32, memory: Handle) -> Handle;
    pub(super) fn GetClipboardData(format: u32) -> Handle;
    pub(super) fn RegisterClipboardFormatW(name: *const u16) -> u32;
}

#[link(name = "gdi32", kind = "raw-dylib")]
unsafe extern "system" {
    pub(super) fn StretchDIBits(
        dc: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        x_source: i32,
        y_source: i32,
        source_width: i32,
        source_height: i32,
        bits: *const c_void,
        info: *const BitmapInfo,
        usage: u32,
        operation: u32,
    ) -> i32;
    pub(super) fn CreateBitmap(
        width: i32,
        height: i32,
        planes: u32,
        bit_count: u32,
        bits: *const c_void,
    ) -> Handle;
    pub(super) fn DeleteObject(object: Handle) -> i32;
}

#[link(name = "kernel32", kind = "raw-dylib")]
unsafe extern "system" {
    pub(super) fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut Handle) -> i32;
    pub(super) fn GlobalAlloc(flags: u32, bytes: usize) -> Handle;
    pub(super) fn GlobalFree(memory: Handle) -> Handle;
    pub(super) fn GlobalLock(memory: Handle) -> *mut c_void;
    pub(super) fn GlobalUnlock(memory: Handle) -> i32;
    pub(super) fn GlobalSize(memory: Handle) -> usize;
}
