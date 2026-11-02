//! What a [`WindowHost`](pltrs::gui::WindowHost) reports, as the tuples
//! the `pltrs` backend dispatches: the event's name first, then its
//! window and what it says.

use pltrs::gui::{CanvasInput, HostEvent, HostedWindowId, InputKind};

use crate::binding::prelude::*;

/// The name of an input kind.
fn kind_name(kind: &InputKind) -> &'static str {
    match kind {
        InputKind::Enter => "enter",
        InputKind::Leave => "leave",
        InputKind::Motion => "motion",
        InputKind::Press { .. } => "press",
        InputKind::Release { .. } => "release",
        InputKind::Scroll { .. } => "scroll",
        InputKind::KeyPress(_) => "key_press",
        InputKind::KeyRelease(_) => "key_release",
    }
}

/// `(kind, window, x, y, modifiers, buttons, detail)` of canvas input,
/// `x` and `y` `None` when the pointer was never over the canvas; the
/// detail is `(button, dblclick)` for a press, the button for a release,
/// the wheel steps for a scroll, the key's name for keys and `None`
/// otherwise.
fn input_into_py(
    py: Python<'_>,
    window: HostedWindowId,
    input: CanvasInput,
) -> PyResult<Bound<'_>> {
    let (x, y) = input
        .position
        .map_or((None, None), |at| (Some(at.x), Some(at.y)));
    let detail = match input.kind {
        InputKind::Press { button, double } => (button, double).into_py(py)?,
        InputKind::Release { button } => button.into_py(py)?,
        InputKind::Scroll { step } => step.into_py(py)?,
        InputKind::KeyPress(ref key) | InputKind::KeyRelease(ref key) => key.into_py(py)?,
        InputKind::Enter | InputKind::Leave | InputKind::Motion => ().into_py(py)?,
    };
    let kind = kind_name(&input.kind);
    (
        kind,
        window.to_raw(),
        x,
        y,
        input.modifiers,
        input.buttons,
        detail,
    )
        .into_py(py)
}

impl IntoPy for HostEvent {
    /// The event as a tuple naming it first: `("resize", window, width,
    /// height, scale_factor)`, canvas input (see [`input_into_py`]),
    /// `("tool", window, name)`, `("close", window)`, `("file", window,
    /// path or None)` and `("timer", timer)`.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::Resized {
                window,
                width,
                height,
                scale_factor,
            } => ("resize", window.to_raw(), width, height, scale_factor).into_py(py),
            Self::Input { window, input } => input_into_py(py, window, input),
            Self::ToolClicked { window, name } => ("tool", window.to_raw(), name).into_py(py),
            Self::CloseRequested { window } => ("close", window.to_raw()).into_py(py),
            Self::FileChosen { window, path } => {
                let path = path.map(|path| path.to_string_lossy().into_owned());
                ("file", window.to_raw(), path).into_py(py)
            }
            Self::Timer(timer) => ("timer", timer.to_raw()).into_py(py),
        }
    }
}
