//! A small retained-mode UI engine used for plt-rs figure windows.
//!
//! # Overview
//!
//! * A UI is a tree of [`Widget`]s held in [`WidgetPod`]s. Containers own
//!   their children: [`Flex`] (rows and columns), [`Form`] (labelled
//!   fields on an aligned label column, grouped into titled cards) and
//!   [`Stack`] (pages of which one shows, e.g. under [`Tabs`]). Leaves
//!   paint and react to input: [`Button`], [`ToolButton`] (a vector
//!   [`Icon`] that can be checked or disabled, with a tooltip),
//!   [`Label`], [`Separator`], [`TextInput`] (a one-line editable field),
//!   [`List`] (scrollable rows), [`Choice`] (a segmented control),
//!   [`Dropdown`] (a button opening a list of options), [`Slider`],
//!   [`Checkbox`],
//!   [`Swatch`] (a colour sample), [`Keycaps`] (keyboard shortcuts) and
//!   [`Surface`] (an application-rendered image, e.g. a figure, with an
//!   optional selection rectangle).
//! * [`Ui`] owns the tree and a window-sized framebuffer. It lays the tree
//!   out (measure, then arrange), paints it with `plt-raster` (text comes
//!   from `plt-text` and the bundled fonts), and routes input: hover
//!   tracking, pointer capture while a button is held, popups, tooltips
//!   and the keyboard.
//! * Every control works from the keyboard, following Qt and GTK: Tab
//!   and Shift+Tab move the focus in tree order (a focus ring shows where
//!   it is), Space and Enter press buttons, arrows move through choices,
//!   lists, sliders and tabs, Enter presses the default button
//!   ([`Button::primary`]) when the focused widget leaves it unused, and
//!   Ctrl+Tab switches tabs. Text fields edit as usual, with the
//!   clipboard and undo.
//! * Widgets talk to the application with messages of a type `M` of its
//!   choosing, returned by [`Ui::handle`].
//! * [`UiWindow`] connects a [`Ui`] to a `plt-platform` window, which it
//!   can size to fit its content, and to the platform's clipboard.
//!
//! All lengths are window pixels (physical pixels); widgets scale the
//! [`Theme`]'s logical lengths by the window's scale factor. The theme
//! holds every colour and metric, and a spacing scale ([`Spacing`]) for
//! padding and gaps.
//!
//! ```
//! use plt_platform::{PhysicalSize, WindowEvent, PhysicalPosition, MouseButton, ButtonState, Modifiers};
//! use plt_ui::{Button, Flex, Surface, Ui};
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
//! # Ok::<(), plt_ui::UiError>(())
//! ```

#![forbid(unsafe_code)]

mod canvas;
mod error;
mod event;
mod geometry;
mod icon;
mod text;
mod theme;
mod ui;
mod widget;
mod widgets;
mod window;

pub use canvas::Canvas;
pub use error::UiError;
pub use event::UiEvent;
pub use geometry::{Constraints, Rect, Size};
pub use icon::Icon;
pub use text::{FontFamily, TextLayout, TextStyle};
pub use theme::{Spacing, Theme};
pub use ui::Ui;
pub use widget::{
    ClipboardRequest, EventCx, Focus, LayoutCx, PaintCx, Status, Widget, WidgetId, WidgetPod,
};
pub use widgets::{
    Align, Axis, Button, Checkbox, Choice, Dropdown, Flex, Form, Keycaps, Label, List, ListItem,
    Overflow, Separator, Slider, Stack, Surface, SurfaceEvent, Swatch, Tabs, TextInput, Tone,
    ToolButton,
};
pub use window::UiWindow;
