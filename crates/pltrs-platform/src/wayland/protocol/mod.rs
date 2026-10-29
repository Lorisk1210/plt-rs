//! The Wayland interfaces this backend speaks: their requests, as
//! functions building [`Request`](super::wire::Request)s, and their
//! events, parsed into [`Event`]s.
//!
//! * [`core`]: `wl_display`, `wl_registry`, `wl_callback`,
//!   `wl_compositor`, `wl_surface`, `wl_region`, `wl_subcompositor`,
//!   `wl_subsurface`, `wl_shm`, `wl_shm_pool`, `wl_buffer`, `wl_output`.
//! * [`shell`]: `xdg_wm_base`, `xdg_surface`, `xdg_toplevel`,
//!   `zxdg_decoration_manager_v1`, `xdg_activation_v1`.
//! * [`input`]: `wl_seat`, `wl_pointer`, `wl_keyboard`,
//!   `wp_cursor_shape_manager_v1`.
//! * [`data`]: `wl_data_device_manager` and the clipboard objects.
//! * [`scale`]: `wp_viewporter` and `wp_fractional_scale_manager_v1`.

pub(crate) mod core;
pub(crate) mod data;
pub(crate) mod input;
pub(crate) mod scale;
pub(crate) mod shell;

use std::os::fd::OwnedFd;

use super::wire::Args;
use crate::error::Error;

/// The interface of an object, which decides how its events parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Interface {
    /// `wl_display`, object 1.
    Display,
    /// `wl_registry`.
    Registry,
    /// `wl_callback`.
    Callback,
    /// `wl_compositor`.
    Compositor,
    /// `wl_surface`.
    Surface,
    /// `wl_subcompositor`.
    Subcompositor,
    /// `wl_subsurface`.
    Subsurface,
    /// `wl_shm`.
    Shm,
    /// `wl_shm_pool`.
    ShmPool,
    /// `wl_buffer`.
    Buffer,
    /// `wl_output`.
    Output,
    /// `wl_seat`.
    Seat,
    /// `wl_pointer`.
    Pointer,
    /// `wl_keyboard`.
    Keyboard,
    /// `wl_data_device_manager`.
    DataDeviceManager,
    /// `wl_data_device`.
    DataDevice,
    /// `wl_data_source`.
    DataSource,
    /// `wl_data_offer`.
    DataOffer,
    /// `xdg_wm_base`.
    WmBase,
    /// `xdg_surface`.
    XdgSurface,
    /// `xdg_toplevel`.
    Toplevel,
    /// `zxdg_decoration_manager_v1`.
    DecorationManager,
    /// `zxdg_toplevel_decoration_v1`.
    Decoration,
    /// `wp_viewporter`.
    Viewporter,
    /// `wp_viewport`.
    Viewport,
    /// `wp_fractional_scale_manager_v1`.
    FractionalScaleManager,
    /// `wp_fractional_scale_v1`.
    FractionalScale,
    /// `wp_cursor_shape_manager_v1`.
    CursorShapeManager,
    /// `wp_cursor_shape_device_v1`.
    CursorShapeDevice,
    /// `xdg_activation_v1`.
    Activation,
    /// `xdg_activation_token_v1`.
    ActivationToken,
}

/// An event this backend reacts to, from the object it was sent by.
#[derive(Debug)]
pub(crate) enum Event {
    /// `wl_display.error`: a fatal protocol error.
    Error {
        /// The object the error is about.
        object: u32,
        /// The interface-specific error code.
        code: u32,
        /// The compositor's description.
        message: String,
    },
    /// `wl_display.delete_id`: a client object id may be reused.
    DeleteId(u32),
    /// `wl_registry.global`.
    Global {
        /// The global's numeric name, for binding.
        name: u32,
        /// Its interface name.
        interface: String,
        /// The highest version the compositor offers.
        version: u32,
    },
    /// `wl_registry.global_remove`.
    GlobalRemove(u32),
    /// `wl_callback.done`.
    Done,
    /// `wl_buffer.release`: the compositor no longer reads the buffer.
    Release,
    /// `wl_surface.enter`: the surface became visible on an output.
    SurfaceEnter(u32),
    /// `wl_surface.leave`.
    SurfaceLeave(u32),
    /// `wl_surface.preferred_buffer_scale`.
    PreferredBufferScale(i32),
    /// `wl_output.scale`.
    OutputScale(i32),
    /// `wl_seat.capabilities`.
    Capabilities(u32),
    /// `wl_pointer.enter`, in surface coordinates.
    PointerEnter {
        /// The input serial.
        serial: u32,
        /// The surface entered.
        surface: u32,
        /// Pointer position.
        x: f64,
        /// Pointer position.
        y: f64,
    },
    /// `wl_pointer.leave`.
    PointerLeave,
    /// `wl_pointer.motion`, in coordinates of the entered surface.
    Motion {
        /// Pointer position.
        x: f64,
        /// Pointer position.
        y: f64,
    },
    /// `wl_pointer.button`, with a Linux `BTN_*` code.
    Button {
        /// The input serial.
        serial: u32,
        /// Milliseconds, of an unspecified base.
        time: u32,
        /// The `BTN_*` code.
        button: u32,
        /// Pressed, or released.
        pressed: bool,
    },
    /// `wl_pointer.axis`: continuous scrolling in surface units.
    Axis {
        /// 0 vertical, 1 horizontal.
        axis: u32,
        /// The distance, positive down or right.
        value: f64,
    },
    /// `wl_pointer.frame`: the end of a group of pointer events.
    Frame,
    /// `wl_pointer.axis_discrete`: wheel notches (before version 8).
    AxisDiscrete {
        /// 0 vertical, 1 horizontal.
        axis: u32,
        /// Notches, positive down or right.
        discrete: i32,
    },
    /// `wl_pointer.axis_value120`: wheel notches in 120ths.
    AxisValue120 {
        /// 0 vertical, 1 horizontal.
        axis: u32,
        /// 120ths of a notch, positive down or right.
        value120: i32,
    },
    /// `wl_keyboard.keymap`.
    Keymap {
        /// 1 for an XKB text keymap.
        format: u32,
        /// The file holding it.
        fd: OwnedFd,
        /// Its length in bytes, terminating NUL included.
        size: u32,
    },
    /// `wl_keyboard.enter`.
    KeyboardEnter {
        /// The input serial.
        serial: u32,
        /// The surface focused.
        surface: u32,
    },
    /// `wl_keyboard.leave`.
    KeyboardLeave {
        /// The surface that lost the focus.
        surface: u32,
    },
    /// `wl_keyboard.key`, with a Linux `KEY_*` code.
    Key {
        /// The input serial.
        serial: u32,
        /// The `KEY_*` code (XKB keycode minus 8).
        key: u32,
        /// Pressed, or released.
        pressed: bool,
    },
    /// `wl_keyboard.modifiers`: XKB modifier masks and the layout.
    Modifiers {
        /// Modifiers held down.
        depressed: u32,
        /// Modifiers latched for the next key.
        latched: u32,
        /// Modifiers locked (Caps Lock, Num Lock).
        locked: u32,
        /// The active layout.
        group: u32,
    },
    /// `wl_keyboard.repeat_info`.
    RepeatInfo {
        /// Repeats per second (0: no repeat).
        rate: i32,
        /// Milliseconds before repeating starts.
        delay: i32,
    },
    /// `wl_data_device.data_offer`: a new offer object.
    DataOffer(u32),
    /// `wl_data_device.enter`: a drag entered, with its offer (if any).
    DragEnter(u32),
    /// `wl_data_device.selection`: the clipboard's offer, or 0 when empty.
    Selection(u32),
    /// `wl_data_offer.offer`: a MIME type the offer has.
    Offer(String),
    /// `wl_data_source.send`: write the content to the descriptor.
    Send {
        /// The MIME type asked for.
        mime: String,
        /// The descriptor to write to and close.
        fd: OwnedFd,
    },
    /// `wl_data_source.cancelled`: another client took the clipboard.
    Cancelled,
    /// `xdg_wm_base.ping`.
    Ping(u32),
    /// `xdg_surface.configure`: apply the configuration and acknowledge.
    Configure(u32),
    /// `xdg_toplevel.configure`: the suggested size (0: client's choice),
    /// in surface coordinates, and the window states.
    ToplevelConfigure {
        /// Suggested width.
        width: i32,
        /// Suggested height.
        height: i32,
        /// `xdg_toplevel.state` values.
        states: Vec<u32>,
    },
    /// `xdg_toplevel.close`.
    Close,
    /// `xdg_toplevel.wm_capabilities`.
    WmCapabilities(Vec<u32>),
    /// `zxdg_toplevel_decoration_v1.configure`: 1 client side, 2 server
    /// side.
    DecorationMode(u32),
    /// `wp_fractional_scale_v1.preferred_scale`, in 120ths.
    PreferredScale(u32),
    /// `xdg_activation_token_v1.done`.
    ActivationToken(String),
    /// An event this backend does not use, or an unknown opcode.
    Ignored,
}

impl Event {
    /// Parses event `opcode` of an object of `interface` from `args`.
    pub(crate) fn parse(
        interface: Interface,
        opcode: u16,
        args: &mut Args<'_>,
    ) -> Result<Self, Error> {
        match interface {
            Interface::Display
            | Interface::Registry
            | Interface::Callback
            | Interface::Buffer
            | Interface::Surface
            | Interface::Output => core::parse(interface, opcode, args),
            Interface::Seat | Interface::Pointer | Interface::Keyboard => {
                input::parse(interface, opcode, args)
            }
            Interface::DataDevice | Interface::DataOffer | Interface::DataSource => {
                data::parse(interface, opcode, args)
            }
            Interface::WmBase
            | Interface::XdgSurface
            | Interface::Toplevel
            | Interface::Decoration
            | Interface::ActivationToken => shell::parse(interface, opcode, args),
            Interface::FractionalScale => scale::parse(opcode, args),
            Interface::Compositor
            | Interface::Subcompositor
            | Interface::Subsurface
            | Interface::Shm
            | Interface::ShmPool
            | Interface::DataDeviceManager
            | Interface::DecorationManager
            | Interface::Viewporter
            | Interface::Viewport
            | Interface::FractionalScaleManager
            | Interface::CursorShapeManager
            | Interface::CursorShapeDevice
            | Interface::Activation => Ok(Self::Ignored),
        }
    }
}

/// An array argument as native-endian 32-bit words.
fn words(array: &[u8]) -> Vec<u32> {
    array
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&word| u32::from_ne_bytes(word))
        .collect()
}
