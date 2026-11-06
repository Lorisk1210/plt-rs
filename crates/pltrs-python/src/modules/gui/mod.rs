//! `_native.gui`: pltrs (Rust)'s figure windows for the `pltrs` backend
//! (`backends/backend_pltrs.py`): a [`WindowHost`] running the windows,
//! which shows each figure's canvas above a toolbar of the backend's
//! buttons and reports input, resizes, toolbar clicks, close requests,
//! chosen files, the subplot tool's changes (`subplots`) and timers as
//! tuples (`events`), which the backend turns into matplotlib's events.
//!
//! Windows and timers are numbered (`HostedWindowId`, `HostTimerId`);
//! positions are physical pixels from the canvas' top-left corner, as the
//! window system gives them. `headless` makes hosts without a display,
//! whose windows tests drive; `desktop` writes pltrs's desktop entry.

mod desktop;
mod events;
mod headless;
mod subplots;

use std::path::PathBuf;
use std::time::Duration;

use pltrs::figure::Figure;
use pltrs::gui::{
    GuiError, HostEvent, HostTimerId, HostedWindowId, ShortcutGroup, ToolImage, ToolItem,
    WindowHost,
};
use pltrs::image::RgbaImage;
use pltrs_core::Bbox;
use pltrs_platform::CursorIcon;

use crate::binding::prelude::*;
use crate::binding::wrap_thread_bound;

impl From<GuiError> for PyErr {
    /// A `RuntimeError` naming what the window system or the window
    /// refused.
    fn from(err: GuiError) -> Self {
        PyErr::runtime_error(err.to_string())
    }
}

impl IntoPy for WindowHost {
    /// The host's object, tied to the thread that made it: windows are
    /// (macOS runs them on its main thread only, Windows delivers their
    /// messages to the thread that made them).
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_thread_bound(py, self)
    }
}

/// A host on the system's window system.
fn connect() -> PyResult<WindowHost> {
    Ok(WindowHost::connect()?)
}

/// The window numbered `window`.
fn id(window: u64) -> HostedWindowId {
    HostedWindowId::from_raw(window)
}

/// Describes a window titled `title` for `figure`, its canvas `width ×
/// height` logical pixels, with an empty toolbar when `toolbar`; it opens
/// when first shown. Returns its number.
fn add_window(
    host: Native<WindowHost>,
    figure: Figure,
    title: String,
    width: f64,
    height: f64,
    toolbar: bool,
) -> PyResult<u64> {
    let window = host
        .borrow_mut()
        .add_window(&figure, &title, (width, height), toolbar);
    Ok(window.to_raw())
}

/// Opens window `window`, or raises it (`manager.show`).
fn show(host: Native<WindowHost>, window: u64) -> PyResult<()> {
    Ok(host.borrow_mut().show(id(window))?)
}

/// Whether window `window` is open on screen.
fn is_shown(host: Native<WindowHost>, window: u64) -> PyResult<bool> {
    Ok(host.borrow().is_shown(id(window)))
}

/// Closes window `window` and forgets it (`manager.destroy`).
fn close(host: Native<WindowHost>, window: u64) -> PyResult<()> {
    Ok(host.borrow_mut().close(id(window))?)
}

/// The title of window `window`.
fn title(host: Native<WindowHost>, window: u64) -> PyResult<String> {
    Ok(host.borrow().title(id(window))?.to_owned())
}

/// Renames window `window`.
fn set_title(host: Native<WindowHost>, window: u64, title: String) -> PyResult<()> {
    Ok(host.borrow_mut().set_title(id(window), &title)?)
}

/// Asks for the canvas of window `window` to be `width × height`
/// physical pixels (`manager.resize`).
fn resize(host: Native<WindowHost>, window: u64, width: u32, height: u32) -> PyResult<()> {
    Ok(host.borrow_mut().resize(id(window), width, height)?)
}

/// Whether window `window` is full screen.
fn is_fullscreen(host: Native<WindowHost>, window: u64) -> PyResult<bool> {
    Ok(host.borrow().is_fullscreen(id(window)))
}

/// Makes window `window` full screen, or not.
fn set_fullscreen(host: Native<WindowHost>, window: u64, fullscreen: bool) -> PyResult<()> {
    Ok(host.borrow_mut().set_fullscreen(id(window), fullscreen)?)
}

/// The cursor of a lower-case name of matplotlib's `Cursors`.
fn cursor(name: &str) -> PyResult<CursorIcon> {
    Ok(match name {
        "pointer" => CursorIcon::Default,
        "hand" => CursorIcon::Pointer,
        "select_region" => CursorIcon::Crosshair,
        "move" => CursorIcon::Move,
        "wait" => CursorIcon::Wait,
        "resize_horizontal" => CursorIcon::ResizeHorizontal,
        "resize_vertical" => CursorIcon::ResizeVertical,
        other => return Err(PyErr::value_error(format!("no cursor named {other:?}"))),
    })
}

/// Shows matplotlib's cursor `name` (a lower-case name of `Cursors`) over
/// window `window`.
fn set_cursor(host: Native<WindowHost>, window: u64, name: String) -> PyResult<()> {
    Ok(host.borrow_mut().set_cursor(id(window), cursor(&name)?)?)
}

/// Shows the zoom rectangle with corners `(x0, y0)` and `(x1, y1)`
/// (canvas pixels from the top-left corner) over window `window`, or
/// hides it with `None`.
fn set_rubberband(
    host: Native<WindowHost>,
    window: u64,
    corners: Option<(f64, f64, f64, f64)>,
) -> PyResult<()> {
    let rubberband = corners.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1));
    Ok(host.borrow_mut().set_rubberband(id(window), rubberband)?)
}

/// The picture of a toolbar button: matplotlib's toolbar image `name`, or
/// a square RGBA `bitmap` (`side × side × 4`).
fn tool_image(name: Option<String>, bitmap: Option<NdArray<u8>>) -> PyResult<ToolImage> {
    match (name, bitmap) {
        (Some(name), _) => Ok(ToolImage::Named(name)),
        (None, Some(bitmap)) => {
            let [height, width, 4] = bitmap.shape[..] else {
                return Err(PyErr::value_error(
                    "a toolbar bitmap is a height × width × 4 array of RGBA bytes",
                ));
            };
            let image = RgbaImage::new(width, height, bitmap.data)
                .map_err(|err| PyErr::value_error(err.to_string()))?;
            Ok(ToolImage::Bitmap(image))
        }
        (None, None) => Err(PyErr::value_error("a toolbar button needs a picture")),
    }
}

/// Adds the button `name` to group `group` of the toolbar of window
/// `window`, at `position` in the group (at the end with `None`), with
/// matplotlib's toolbar image `image` or the RGBA `bitmap`
/// (`add_toolitem`).
#[expect(
    clippy::too_many_arguments,
    reason = "matplotlib's add_toolitem takes them all at once"
)]
fn add_tool(
    host: Native<WindowHost>,
    window: u64,
    name: String,
    group: String,
    position: Option<usize>,
    tooltip: String,
    image: Option<String>,
    bitmap: Option<NdArray<u8>>,
) -> PyResult<()> {
    let item = ToolItem {
        name,
        group,
        tooltip,
        image: tool_image(image, bitmap)?,
    };
    Ok(host.borrow_mut().add_tool(id(window), item, position)?)
}

/// Whether pltrs draws matplotlib's toolbar image `name` itself.
fn has_tool_image(name: String) -> PyResult<bool> {
    Ok(ToolImage::is_drawn(&name))
}

/// Removes the toolbar buttons called `name` of window `window`.
fn remove_tool(host: Native<WindowHost>, window: u64, name: String) -> PyResult<bool> {
    Ok(host.borrow_mut().remove_tool(id(window), &name)?)
}

/// Checks or unchecks the toolbar buttons called `name`.
fn set_tool_checked(
    host: Native<WindowHost>,
    window: u64,
    name: String,
    checked: bool,
) -> PyResult<()> {
    Ok(host
        .borrow_mut()
        .set_tool_checked(id(window), &name, checked)?)
}

/// Enables or disables the toolbar buttons called `name`.
fn set_tool_enabled(
    host: Native<WindowHost>,
    window: u64,
    name: String,
    enabled: bool,
) -> PyResult<()> {
    Ok(host
        .borrow_mut()
        .set_tool_enabled(id(window), &name, enabled)?)
}

/// The groups of the toolbar of window `window` with the names of their
/// buttons.
fn toolbar_layout(host: Native<WindowHost>, window: u64) -> PyResult<Vec<(String, Vec<String>)>> {
    Ok(host.borrow().toolbar_layout(id(window))?)
}

/// Whether the toolbar button `name` is checked, `None` without one.
fn tool_checked(host: Native<WindowHost>, window: u64, name: String) -> PyResult<Option<bool>> {
    Ok(host.borrow().tool_checked(id(window), &name)?)
}

/// Shows `text` in the status line of window `window`.
fn set_message(host: Native<WindowHost>, window: u64, text: String) -> PyResult<()> {
    Ok(host.borrow_mut().set_message(id(window), &text)?)
}

/// The message of the status line of window `window`.
fn message(host: Native<WindowHost>, window: u64) -> PyResult<String> {
    Ok(host.borrow().message(id(window))?)
}

/// Opens the file chooser of window `window` in `directory` (else in
/// `savefig.directory`) proposing `name`; the choice arrives as a
/// `"file"` event.
fn choose_file(
    host: Native<WindowHost>,
    window: u64,
    directory: Option<PathBuf>,
    name: String,
) -> PyResult<()> {
    Ok(host
        .borrow_mut()
        .choose_file(id(window), directory.as_deref(), &name)?)
}

/// Shows the keyboard shortcuts `groups` lists for window `window`: each
/// group's name, then what each shortcut does and its keys.
fn show_help(host: Native<WindowHost>, window: u64, groups: Vec<ShortcutGroup>) -> PyResult<()> {
    Ok(host.borrow_mut().show_help(id(window), groups)?)
}

/// Puts what window `window` shows of its canvas on the clipboard.
fn copy_to_clipboard(host: Native<WindowHost>, window: u64) -> PyResult<()> {
    Ok(host.borrow_mut().copy_to_clipboard(id(window))?)
}

/// Shows in window `window` what changed on its figure's canvas.
fn present(host: Native<WindowHost>, window: u64) -> PyResult<()> {
    Ok(host.borrow_mut().present(id(window))?)
}

/// A duration of `seconds`, refusing what is no duration.
fn duration(seconds: f64) -> PyResult<Duration> {
    Duration::try_from_secs_f64(seconds)
        .map_err(|_| PyErr::value_error(format!("{seconds} is not a duration in seconds")))
}

/// Starts a timer firing every `interval` seconds, or once unless
/// `repeat`; returns its number.
fn start_timer(host: Native<WindowHost>, interval: f64, repeat: bool) -> PyResult<u64> {
    let timer = host.borrow_mut().start_timer(duration(interval)?, repeat);
    Ok(timer.to_raw())
}

/// Stops timer `timer`; whether it was running.
fn stop_timer(host: Native<WindowHost>, timer: u64) -> PyResult<bool> {
    Ok(host.borrow_mut().stop_timer(HostTimerId::from_raw(timer)))
}

/// Shows what changed in every window, waits up to `timeout` seconds
/// (forever with `None`) for something to happen, and returns what did,
/// as event tuples.
fn pump(host: Native<WindowHost>, timeout: Option<f64>) -> PyResult<Vec<HostEvent>> {
    let timeout = timeout.map(duration).transpose()?;
    Ok(host.borrow_mut().pump(timeout)?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("connect", connect)?;
    module.add("add_window", add_window)?;
    module.add("show", show)?;
    module.add("is_shown", is_shown)?;
    module.add("close", close)?;
    module.add("title", title)?;
    module.add("set_title", set_title)?;
    module.add("resize", resize)?;
    module.add("is_fullscreen", is_fullscreen)?;
    module.add("set_fullscreen", set_fullscreen)?;
    module.add("set_cursor", set_cursor)?;
    module.add("set_rubberband", set_rubberband)?;
    module.add("add_tool", add_tool)?;
    module.add("has_tool_image", has_tool_image)?;
    module.add("remove_tool", remove_tool)?;
    module.add("set_tool_checked", set_tool_checked)?;
    module.add("set_tool_enabled", set_tool_enabled)?;
    module.add("toolbar_layout", toolbar_layout)?;
    module.add("tool_checked", tool_checked)?;
    module.add("set_message", set_message)?;
    module.add("message", message)?;
    module.add("choose_file", choose_file)?;
    module.add("show_help", show_help)?;
    module.add("copy_to_clipboard", copy_to_clipboard)?;
    module.add("present", present)?;
    module.add("start_timer", start_timer)?;
    module.add("stop_timer", stop_timer)?;
    module.add("pump", pump)?;
    subplots::register(module)?;
    desktop::register(module)?;
    headless::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_have_matplotlibs_names() {
        assert_eq!(cursor("select_region").unwrap(), CursorIcon::Crosshair);
        assert_eq!(cursor("hand").unwrap(), CursorIcon::Pointer);
        assert!(cursor("lasso").is_err());
    }

    #[test]
    fn durations_are_seconds() {
        assert_eq!(duration(0.25).unwrap(), Duration::from_millis(250));
        assert!(duration(-1.0).is_err());
        assert!(duration(f64::NAN).is_err());
    }

    #[test]
    fn toolbar_pictures_are_names_or_square_bitmaps() {
        assert_eq!(
            tool_image(Some("home".into()), None).unwrap(),
            ToolImage::Named("home".into())
        );
        let bitmap = NdArray::with_shape(vec![0; 2 * 2 * 4], vec![2, 2, 4]).unwrap();
        assert!(matches!(
            tool_image(None, Some(bitmap)).unwrap(),
            ToolImage::Bitmap(_)
        ));
        let flat = NdArray::with_shape(vec![0; 8], vec![2, 4]).unwrap();
        assert!(tool_image(None, Some(flat)).is_err());
        assert!(tool_image(None, None).is_err());
    }
}
