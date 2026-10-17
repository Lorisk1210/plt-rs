//! A small retained-mode UI engine used for plt-rs figure windows.
//!
//! # Overview
//!
//! * A UI is a tree of [`Widget`]s held in [`WidgetPod`]s. Containers such
//!   as [`Flex`] own their children; leaves such as [`Button`],
//!   [`ToolButton`] (a vector [`Icon`] that can be checked or disabled),
//!   [`Label`], [`Separator`] and [`Surface`] (an application-rendered
//!   image, e.g. a figure, with an optional selection rectangle) paint and
//!   react to input.
//! * [`Ui`] owns the tree and a window-sized framebuffer. It lays the tree
//!   out (measure, then arrange), paints it with `plt-raster` (text comes
//!   from `plt-text` and the bundled fonts), and routes input: hover
//!   tracking, pointer capture while a button is held, keyboard focus.
//! * Widgets talk to the application with messages of a type `M` of its
//!   choosing, returned by [`Ui::handle`].
//! * [`UiWindow`] connects a [`Ui`] to a `plt-platform` window.
//!
//! All lengths are window pixels (physical pixels); widgets scale the
//! [`Theme`]'s logical lengths by the window's scale factor.
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
pub use text::{TextLayout, TextStyle};
pub use theme::Theme;
pub use ui::Ui;
pub use widget::{EventCx, LayoutCx, PaintCx, Status, Widget, WidgetId, WidgetPod};
pub use widgets::{Axis, Button, Flex, Label, Separator, Surface, SurfaceEvent, ToolButton};
pub use window::UiWindow;
