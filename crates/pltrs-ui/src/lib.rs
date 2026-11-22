//! A small retained-mode UI engine for the figure windows of pltrs.
//!
//! # Overview
//!
//! * A UI is a tree of [`Widget`]s in [`WidgetPod`]s. Containers own their
//!   children:
//!   - [`Flex`]: rows and columns.
//!   - [`Form`]: labeled fields with an aligned label column, in titled
//!     cards.
//!   - [`Stack`]: pages of which one shows, for example under [`Tabs`].
//!   - [`Scroll`]: a view that scrolls a taller widget.
//! * Leaves paint and react to input:
//!   - [`Button`], [`Label`], [`Separator`], [`Slider`], [`Checkbox`].
//!   - [`ToolButton`]: a vector [`Icon`] or a full-color [`Picture`], with
//!     a tooltip. It can be checked or disabled.
//!   - [`TextInput`]: an editable field of one line.
//!   - [`List`]: rows that scroll.
//!   - [`Choice`]: a segmented control.
//!   - [`Dropdown`]: a button that opens a list of options.
//!   - [`Swatch`]: a color sample.
//!   - [`Keycaps`]: keyboard shortcuts.
//!   - [`Surface`]: an image that the application renders, for example a
//!     figure, with an optional selection rectangle.
//! * [`ColorIcon`]s are full-color vector icons for applications, at any
//!   size. They have filled shapes in solid colors or
//!   [`LinearGradient`]s, outlines and blurred drop [`Shadow`]s.
//! * [`Ui`] owns the tree and a framebuffer of the window size. It lays
//!   out the tree (measure, then arrange) and paints it with
//!   `pltrs-raster`. The text comes from `pltrs-text` and the included
//!   fonts. It routes input: hover, pointer capture while a button is
//!   held, popups, tooltips and the keyboard.
//! * All controls work from the keyboard, as in Qt and GTK. Tab and
//!   Shift+Tab move the focus in tree order, and a focus ring shows it.
//!   Space and Enter press buttons. The arrows move through choices,
//!   lists, sliders and tabs. If the focused widget does not use Enter,
//!   Enter presses the default button ([`Button::primary`]). Ctrl+Tab
//!   changes the tab. Text fields have the usual editing, the clipboard
//!   and undo.
//! * Widgets send messages of a type `M` to the application.
//!   [`Ui::handle`] returns them.
//! * [`UiWindow`] connects a [`Ui`] to a `pltrs-platform` window and to
//!   the clipboard of the platform. It can size the window to fit the
//!   content. [`TitleBarPainter`] draws the title bars that the platform
//!   adds where the desktop leaves decorations to applications (Wayland
//!   without server-side decorations).
//!
//! All lengths are window pixels (physical pixels). Widgets scale the
//! logical lengths of the [`Theme`] by the scale factor of the window. The
//! theme holds all colors and metrics, and a spacing scale ([`Spacing`])
//! for padding and gaps.
//!
//! ```
//! use pltrs_platform::{PhysicalSize, WindowEvent, PhysicalPosition, MouseButton, ButtonState, Modifiers};
//! use pltrs_ui::{Button, Flex, Surface, Ui};
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Message {
//!     Home,
//! }
//!
//! let root = Flex::column()
//!     .with_flex_child(Surface::new(|_| None), 1.0)
//!     .with_child(Flex::row().with_child(Button::new("Home", Message::Home)));
//! let mut ui = Ui::new(root, PhysicalSize::new(400, 300), 1.0)?;
//! ui.paint()?;
//! // Click the button in the bottom-left corner.
//! let position = PhysicalPosition::new(10.0, 290.0);
//! let click = |state| WindowEvent::MouseInput {
//!     button: MouseButton::Left,
//!     state,
//!     position,
//!     modifiers: Modifiers::NONE,
//!     click_count: 1,
//! };
//! ui.handle(&click(ButtonState::Pressed))?;
//! assert_eq!(ui.handle(&click(ButtonState::Released))?, [Message::Home]);
//! # Ok::<(), pltrs_ui::UiError>(())
//! ```

#![forbid(unsafe_code)]

mod canvas;
mod color_icon;
mod error;
mod event;
mod geometry;
mod icon;
mod picture;
mod text;
mod theme;
mod title_bar;
mod ui;
mod widget;
mod widgets;
mod window;

/// The harness of pltrs's fuzz tests (`pltrs_core::fuzz`), driving the
/// unit tests that fuzz the widgets with random input.
#[cfg(test)]
use pltrs_core::fuzz;

pub use canvas::Canvas;
pub use color_icon::{ColorIcon, LinearGradient, Paint, Shadow};
pub use error::UiError;
pub use event::UiEvent;
pub use geometry::{Constraints, Rect, Size};
pub use icon::Icon;
pub use picture::Picture;
pub use text::{FontFamily, TextLayout, TextStyle};
pub use theme::{Spacing, Theme, TooltipStyle};
pub use title_bar::TitleBarPainter;
pub use ui::Ui;
pub use widget::{
    ClipboardRequest, EventCx, Focus, LayoutCx, PaintCx, Status, Widget, WidgetId, WidgetPod,
};
pub use widgets::{
    Align, Axis, Button, Checkbox, Choice, Dropdown, Flex, Form, Keycaps, Label, List, ListItem,
    Overflow, Scroll, Separator, Slider, Stack, Surface, SurfaceEvent, Swatch, Tabs, TextInput,
    Tone, ToolButton, ToolIcon,
};
pub use window::UiWindow;
