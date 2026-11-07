//! The window class and its procedure, windows, the message loop, and
//! keyboard and pointer queries.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::OnceLock;

use super::ffi::{self, Handle, MinMaxInfo, Msg, Point, Rect, WindowClassEx, WindowPlacement};
use super::{Hwnd, check, last_error};
use crate::error::Error;
use crate::geometry::PhysicalSize;
use crate::windows::convert::{single_character, to_wide};
use crate::windows::keys::{HeldModifiers, KeyMessage, Typed, release_control_and_alt};
use crate::windows::message::{Message, ScreenRect, is_key, key_message, wm};
use crate::windows::procedure::{self, Reply};

/// The name of the window class of pltrs windows.
const CLASS_NAME: &str = "pltrs-window";
/// `WM_KEYLAST`: the last keyboard message.
const WM_KEYLAST: u32 = 0x0109;
/// `GWL_EXSTYLE`.
const GWL_EXSTYLE: i32 = -20;

/// The window class of pltrs windows, registered once per process.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowClass {
    /// The module the class (and its procedure) belongs to.
    instance: Handle,
}

impl WindowClass {
    /// The class, registered on first use.
    pub(crate) fn get() -> Result<Self, Error> {
        static CLASS: OnceLock<Result<WindowClass, Error>> = OnceLock::new();
        CLASS.get_or_init(Self::register).clone()
    }

    fn register() -> Result<Self, Error> {
        let mut instance = 0;
        let flags = ffi::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
            | ffi::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        let address = (window_procedure as *const ()).cast::<u16>();
        // SAFETY: the address lies in this module's code, and the flags
        // leave the module's reference count alone, so nothing is to be
        // released; `instance` receives the handle.
        let found = unsafe { ffi::GetModuleHandleExW(flags, address, &raw mut instance) };
        check(found, "GetModuleHandleExW")?;
        let name = to_wide(CLASS_NAME);
        let class = WindowClassEx {
            size: size_of::<WindowClassEx>() as u32,
            style: 0,
            procedure: Some(window_procedure),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: 0,
            cursor: 0,
            background: 0,
            menu_name: ptr::null(),
            class_name: name.as_ptr(),
            small_icon: 0,
        };
        // SAFETY: `class` is a complete WNDCLASSEXW whose name outlives
        // the call (the system copies it).
        if unsafe { ffi::RegisterClassExW(&raw const class) } == 0 {
            return Err(last_error("RegisterClassExW"));
        }
        Ok(Self { instance })
    }

    /// Creates a hidden top-level window titled `title`, with a title bar
    /// and sizing borders, where the system places windows.
    pub(crate) fn create_window(self, title: &str) -> Result<Hwnd, Error> {
        let default = ffi::CW_USEDEFAULT;
        self.create(title, ffi::WS_OVERLAPPEDWINDOW, 0, default)
    }

    /// Creates a message-only window: invisible, the target of the
    /// messages that wake the event loop.
    pub(crate) fn create_message_window(self) -> Result<Hwnd, Error> {
        self.create("", 0, ffi::HWND_MESSAGE, 0)
    }

    fn create(self, title: &str, style: u32, parent: Handle, place: i32) -> Result<Hwnd, Error> {
        let class = to_wide(CLASS_NAME);
        let title = to_wide(title);
        // SAFETY: both strings are NUL-terminated and outlive the call, the
        // class is registered for `self.instance`, and no creation data is
        // passed.
        let window = unsafe {
            ffi::CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                style,
                place,
                place,
                place,
                place,
                parent,
                0,
                self.instance,
                ptr::null_mut(),
            )
        };
        if window == 0 {
            return Err(last_error("CreateWindowExW"));
        }
        Ok(Hwnd(window))
    }
}

/// The procedure of pltrs windows: decodes the message, lets
/// [`procedure::handle`] act on it, and answers the system.
extern "system" fn window_procedure(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let decoded = if message == wm::DPICHANGED && lparam != 0 {
        // SAFETY: the lParam of WM_DPICHANGED points to the suggested
        // window rectangle, valid during the message.
        let rect = unsafe { *(lparam as *const Rect) };
        Message::dpi_changed(wparam, screen_rect(rect))
    } else {
        Message::decode(message, wparam, lparam, message_time())
    };
    // A panic must not unwind into the system; the message then gets the
    // default handling.
    let reply = catch_unwind(AssertUnwindSafe(|| {
        procedure::handle(Hwnd(window), decoded)
    }))
    .unwrap_or(Reply::Default);
    match reply {
        Reply::Handled(result) => result,
        Reply::MinTrackSize { width, height } if message == wm::GETMINMAXINFO && lparam != 0 => {
            // SAFETY: the lParam of WM_GETMINMAXINFO points to a
            // MINMAXINFO the window may change during the message.
            let info = unsafe { &mut *(lparam as *mut MinMaxInfo) };
            info.min_track_size = Point {
                x: width,
                y: height,
            };
            0
        }
        // SAFETY: the system's handling of this window's message, with the
        // parameters the system passed.
        _ => unsafe { ffi::DefWindowProcW(window, message, wparam, lparam) },
    }
}

/// A `RECT` as a [`ScreenRect`].
const fn screen_rect(rect: Rect) -> ScreenRect {
    ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

/// What a window looked like before it was made full screen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Restore {
    style: u32,
    placement: WindowPlacement,
}

impl Hwnd {
    /// Destroys the window.
    pub(crate) fn destroy(self) -> Result<(), Error> {
        // SAFETY: a handle-only call.
        check(unsafe { ffi::DestroyWindow(self.0) }, "DestroyWindow")
    }

    /// Shows the window and activates it.
    pub(crate) fn show(self) {
        // SAFETY: a handle-only call; the result is the previous
        // visibility, not an error.
        unsafe { ffi::ShowWindow(self.0, ffi::SW_SHOWNORMAL) };
    }

    /// Restores the window if minimized, and asks to bring it to the
    /// foreground (which Windows may refuse to an inactive process).
    pub(crate) fn raise(self) {
        // SAFETY: handle-only calls, whose results are states, not
        // errors.
        unsafe {
            if ffi::IsIconic(self.0) != 0 {
                ffi::ShowWindow(self.0, ffi::SW_RESTORE);
            }
            ffi::SetForegroundWindow(self.0);
        }
    }

    /// Changes the title.
    pub(crate) fn set_title(self, title: &str) -> Result<(), Error> {
        let title = to_wide(title);
        // SAFETY: the title is NUL-terminated and outlives the call.
        let done = unsafe { ffi::SetWindowTextW(self.0, title.as_ptr()) };
        check(done, "SetWindowTextW")
    }

    /// The window's DPI (0 for a stale handle).
    pub(crate) fn dpi(self) -> u32 {
        // SAFETY: a handle-only call.
        unsafe { ffi::GetDpiForWindow(self.0) }
    }

    /// The window size, borders and title bar included, that gives a
    /// client area of `client` at `dpi`, with the window's current style.
    pub(crate) fn outer_size(self, client: PhysicalSize, dpi: u32) -> Result<(i32, i32), Error> {
        let too_large = Error::InvalidArgument("window size exceeds Windows' limits");
        let width = i32::try_from(client.width).map_err(|_| too_large.clone())?;
        let height = i32::try_from(client.height).map_err(|_| too_large)?;
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        // SAFETY: handle-only calls, then `rect` is a RECT to adjust.
        let adjusted = unsafe {
            let style = ffi::GetWindowLongW(self.0, ffi::GWL_STYLE) as u32;
            let ex_style = ffi::GetWindowLongW(self.0, GWL_EXSTYLE) as u32;
            ffi::AdjustWindowRectExForDpi(&raw mut rect, style, 0, ex_style, dpi)
        };
        check(adjusted, "AdjustWindowRectExForDpi")?;
        Ok((rect.right - rect.left, rect.bottom - rect.top))
    }

    /// Changes the window size (borders included), keeping its position.
    pub(crate) fn resize(self, (width, height): (i32, i32)) -> Result<(), Error> {
        let flags = ffi::SWP_NOMOVE | ffi::SWP_NOZORDER | ffi::SWP_NOACTIVATE;
        // SAFETY: a handle-only call.
        let done = unsafe { ffi::SetWindowPos(self.0, 0, 0, 0, width, height, flags) };
        check(done, "SetWindowPos")
    }

    /// Moves and sizes the window to `rect` (borders included), if the
    /// system lets it.
    pub(crate) fn place(self, rect: ScreenRect) {
        let flags = ffi::SWP_NOZORDER | ffi::SWP_NOACTIVATE;
        let (width, height) = (rect.width(), rect.height());
        // SAFETY: a handle-only call; on failure the window keeps its
        // place and size.
        unsafe { ffi::SetWindowPos(self.0, 0, rect.left, rect.top, width, height, flags) };
    }

    /// Makes the window cover its monitor without borders; returns what
    /// restores it.
    pub(crate) fn enter_fullscreen(self) -> Result<Restore, Error> {
        let mut placement = WindowPlacement {
            length: size_of::<WindowPlacement>() as u32,
            ..WindowPlacement::default()
        };
        let mut monitor = ffi::MonitorInfo {
            size: size_of::<ffi::MonitorInfo>() as u32,
            ..ffi::MonitorInfo::default()
        };
        // SAFETY: handle-only calls, and structures sized as their `length`
        // and `size` fields say, for the system to fill.
        let style = unsafe {
            check(
                ffi::GetWindowPlacement(self.0, &raw mut placement),
                "GetWindowPlacement",
            )?;
            let nearest = ffi::MonitorFromWindow(self.0, ffi::MONITOR_DEFAULTTONEAREST);
            check(
                ffi::GetMonitorInfoW(nearest, &raw mut monitor),
                "GetMonitorInfoW",
            )?;
            ffi::GetWindowLongW(self.0, ffi::GWL_STYLE) as u32
        };
        let popup = (style & !ffi::WS_OVERLAPPEDWINDOW) | ffi::WS_POPUP;
        let area = monitor.monitor;
        let flags = ffi::SWP_NOOWNERZORDER | ffi::SWP_FRAMECHANGED;
        // SAFETY: handle-only calls.
        let done = unsafe {
            ffi::SetWindowLongW(self.0, ffi::GWL_STYLE, popup as i32);
            ffi::SetWindowPos(
                self.0,
                0,
                area.left,
                area.top,
                area.right - area.left,
                area.bottom - area.top,
                flags,
            )
        };
        check(done, "SetWindowPos")?;
        Ok(Restore { style, placement })
    }

    /// Gives a full-screen window back the style and place `restore`
    /// recorded.
    pub(crate) fn leave_fullscreen(self, restore: &Restore) -> Result<(), Error> {
        let flags = ffi::SWP_NOMOVE
            | ffi::SWP_NOSIZE
            | ffi::SWP_NOZORDER
            | ffi::SWP_NOOWNERZORDER
            | ffi::SWP_FRAMECHANGED;
        // SAFETY: handle-only calls, and a WINDOWPLACEMENT that
        // GetWindowPlacement filled.
        unsafe {
            ffi::SetWindowLongW(self.0, ffi::GWL_STYLE, restore.style as i32);
            check(
                ffi::SetWindowPlacement(self.0, &raw const restore.placement),
                "SetWindowPlacement",
            )?;
            check(
                ffi::SetWindowPos(self.0, 0, 0, 0, 0, 0, flags),
                "SetWindowPos",
            )
        }
    }
}

/// Makes the windows this thread creates from now on per-monitor DPI
/// aware (version 2 where available): they get physical pixel sizes and
/// `WM_DPICHANGED` when they move to a monitor of another DPI.
pub(crate) fn use_per_monitor_dpi() {
    // SAFETY: handle-only calls, with predefined contexts; on failure the
    // thread keeps its awareness.
    unsafe {
        if ffi::SetThreadDpiAwarenessContext(ffi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            ffi::SetThreadDpiAwarenessContext(ffi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
        }
    }
}

/// Dispatches every message queued for this thread to its window's
/// procedure, translating key presses into character messages first.
pub(crate) fn dispatch_messages() {
    let mut msg = Msg::default();
    // SAFETY: `msg` is a MSG for PeekMessageW to fill, then passed back
    // as filled.
    unsafe {
        while ffi::PeekMessageW(&raw mut msg, 0, 0, 0, ffi::PM_REMOVE) != 0 {
            ffi::TranslateMessage(&raw const msg);
            ffi::DispatchMessageW(&raw const msg);
        }
    }
}

/// Waits until a message is queued for this thread, or `timeout_ms`
/// milliseconds pass ([`INFINITE`](crate::windows::convert::INFINITE):
/// no limit).
pub(crate) fn wait_for_messages(timeout_ms: u32) -> Result<(), Error> {
    // SAFETY: no object handles are passed (a count of 0).
    let result = unsafe {
        ffi::MsgWaitForMultipleObjectsEx(
            0,
            ptr::null(),
            timeout_ms,
            ffi::QS_ALLINPUT,
            ffi::MWMO_INPUTAVAILABLE,
        )
    };
    if result == ffi::WAIT_FAILED {
        return Err(last_error("MsgWaitForMultipleObjectsEx"));
    }
    Ok(())
}

/// Posts the wake-up message to `window`, from any thread.
pub(crate) fn post_wake(window: Hwnd) -> Result<(), Error> {
    // SAFETY: a handle-only call.
    let posted = unsafe { ffi::PostMessageW(window.0, wm::WAKE, 0, 0) };
    check(posted, "PostMessageW")
}

/// The time of the message being handled, in milliseconds.
fn message_time() -> u32 {
    // SAFETY: a call without arguments.
    unsafe { ffi::GetMessageTime() as u32 }
}

/// The double-click time the user chose, in milliseconds.
pub(crate) fn double_click_time() -> u32 {
    // SAFETY: a call without arguments.
    unsafe { ffi::GetDoubleClickTime() }
}

/// The modifier keys held when the message being handled was sent.
pub(crate) fn held_modifiers() -> HeldModifiers {
    // SAFETY: a call with a key code; a negative state means "down".
    HeldModifiers::from_keys(|key| unsafe { ffi::GetKeyState(i32::from(key)) } < 0)
}

/// The key message queued next for `window`, if the next keyboard
/// message is one (and not the character messages of the key being
/// handled).
pub(crate) fn next_key_message(window: Hwnd) -> Option<KeyMessage> {
    let mut msg = Msg::default();
    // SAFETY: `msg` is a MSG to fill; the message stays queued.
    let found = unsafe {
        ffi::PeekMessageW(
            &raw mut msg,
            window.0,
            wm::KEYDOWN,
            WM_KEYLAST,
            ffi::PM_NOREMOVE,
        )
    };
    (found != 0 && is_key(msg.message))
        .then(|| key_message(msg.message, msg.wparam, msg.lparam, msg.time))
}

/// Takes the character messages queued right after the key press being
/// handled (its translation, with `TranslateMessage`): what it typed,
/// and the accent of a dead key.
pub(crate) fn take_typed(window: Hwnd) -> Typed {
    let mut units = Vec::new();
    let mut dead = None;
    loop {
        let mut next = Msg::default();
        // SAFETY: `next` is a MSG to fill; the message stays queued.
        let found = unsafe {
            ffi::PeekMessageW(
                &raw mut next,
                window.0,
                wm::KEYDOWN,
                WM_KEYLAST,
                ffi::PM_NOREMOVE,
            )
        };
        if found == 0 || !matches!(next.message, wm::CHAR | wm::DEADCHAR) {
            return Typed::from_messages(&units, dead);
        }
        let mut taken = Msg::default();
        // SAFETY: `taken` is a MSG to fill; this removes the character
        // message just seen, the window's first of its kind.
        let removed = unsafe {
            ffi::PeekMessageW(
                &raw mut taken,
                window.0,
                next.message,
                next.message,
                ffi::PM_REMOVE,
            )
        };
        if removed == 0 {
            return Typed::from_messages(&units, dead);
        }
        if taken.message == wm::CHAR {
            units.push(taken.wparam as u16);
        } else {
            dead = Some(taken.wparam as u16);
        }
    }
}

/// The character the keyboard layout gives `key`, with Shift and Caps
/// Lock applied, and `AltGr` if `alt_gr`, but neither Control nor Alt;
/// the layout's dead-key state is left as it is.
pub(crate) fn layout_character(key: KeyMessage, alt_gr: bool) -> Option<char> {
    let mut state = [0_u8; 256];
    // SAFETY: `state` has the 256 bytes GetKeyboardState fills.
    if unsafe { ffi::GetKeyboardState(state.as_mut_ptr()) } == 0 {
        return None;
    }
    if !alt_gr {
        release_control_and_alt(&mut state);
    }
    let mut buffer = [0_u16; 8];
    // SAFETY: `state` has 256 bytes, `buffer` the length passed, and the
    // flag keeps the keyboard state (dead keys) untouched.
    let count = unsafe {
        ffi::ToUnicodeEx(
            u32::from(key.vk),
            u32::from(key.scancode),
            state.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            ffi::TO_UNICODE_KEEP_STATE,
            ffi::GetKeyboardLayout(0),
        )
    };
    single_character(&buffer, count)
}

/// Asks for `WM_MOUSELEAVE` when the pointer leaves `window`'s client
/// area.
pub(crate) fn track_leave(window: Hwnd) {
    let mut track = ffi::TrackMouseEvent {
        size: size_of::<ffi::TrackMouseEvent>() as u32,
        flags: ffi::TME_LEAVE,
        window: window.0,
        hover_time: 0,
    };
    // SAFETY: `track` is a TRACKMOUSEEVENT sized as its `size` says; a
    // failure only loses the leave notification.
    unsafe { ffi::TrackMouseEvent(&raw mut track) };
}

/// Sends the pointer's input to `window` while a button is held, even
/// outside it.
pub(crate) fn capture(window: Hwnd) {
    // SAFETY: a handle-only call; the result is the previous capture.
    unsafe { ffi::SetCapture(window.0) };
}

/// Ends the pointer capture.
pub(crate) fn release_capture() {
    // SAFETY: a call without arguments; without a capture it does
    // nothing.
    unsafe { ffi::ReleaseCapture() };
}

/// A screen position in `window`'s client coordinates.
pub(crate) fn screen_to_client(window: Hwnd, (x, y): (i32, i32)) -> (i32, i32) {
    let mut point = Point { x, y };
    // SAFETY: `point` is a POINT to convert in place.
    unsafe { ffi::ScreenToClient(window.0, &raw mut point) };
    (point.x, point.y)
}

/// Where the pointer is, in `window`'s client coordinates; `None` when
/// the system cannot tell (another desktop has the input).
pub(crate) fn cursor_in_client(window: Hwnd) -> Option<(i32, i32)> {
    let mut point = Point { x: 0, y: 0 };
    // SAFETY: `point` is a POINT the call fills in.
    let found = unsafe { ffi::GetCursorPos(&raw mut point) } != 0;
    found.then(|| screen_to_client(window, (point.x, point.y)))
}
