//! A whole backend against the scripted compositor, which answers every
//! request as usual while random events reach the backend's objects:
//! made-up arguments, descriptors (keymaps mutated, empty files, pipes),
//! garbage bytes, protocol errors and a hang-up. Meanwhile the backend
//! polls, presents, resizes, copies, pastes, changes cursors and titles,
//! and closes its window. Errors are fine; panics, hangs and unbounded
//! allocations are not.

use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use std::time::{Duration, Instant};

use super::{Arg, args, message, word};
use crate::backend::Backend;
use crate::blocking::Blocking;
use crate::decoration::{ButtonLayout, TitleBarAction, TitleBarSettings};
use crate::frame::OwnedFrame;
use crate::fuzz::{Gen, fuzz};
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::wayland::fake::{Fake, Options};
use crate::wayland::fixtures::DE_RU_KEYMAP;
use crate::window::{CursorIcon, WindowOptions};

/// A descriptor slipped into an event.
#[derive(Debug, Clone)]
enum Descriptor {
    /// A file holding these bytes.
    File(Vec<u8>),
    /// The read end of an empty pipe.
    Pipe,
}

/// Something the compositor does.
#[derive(Debug, Clone)]
enum Action {
    /// An event to the `object`-th object the fake knows (or a made-up id).
    Event {
        object: usize,
        opcode: u16,
        args: Vec<Arg>,
        fd: Option<Descriptor>,
    },
    /// Bytes that may not even be a message.
    Garbage(Vec<u8>),
}

/// Something the application does.
#[derive(Debug, Clone, Copy)]
enum Call {
    Poll,
    Present(u32, u32),
    Resize(u32, u32),
    Title,
    Cursor,
    Fullscreen(bool),
    Copy,
    Paste,
    Raise,
    Close,
}

/// One run: the compositor's options, its actions, the application's
/// calls between them, and whether the compositor hangs up at the end.
#[derive(Debug)]
struct Run {
    options: Options,
    steps: Vec<(Vec<Action>, Call)>,
    hang_up: bool,
}

fn descriptor(g: &mut Gen) -> Descriptor {
    match g.below(3) {
        0 => Descriptor::Pipe,
        1 => Descriptor::File((0..g.len(64)).map(|_| g.below(256) as u8).collect()),
        _ => {
            let edited = g.mutate(DE_RU_KEYMAP, &["key", "{", "};", "[", "<A>"]);
            Descriptor::File(edited.into_bytes())
        }
    }
}

fn action(g: &mut Gen) -> Action {
    if g.chance(0.05) {
        return Action::Garbage((0..g.len(40)).map(|_| g.below(256) as u8).collect());
    }
    Action::Event {
        object: g.below(48),
        opcode: g.below(11) as u16,
        args: args(g),
        fd: g.chance(0.15).then(|| descriptor(g)),
    }
}

fn call(g: &mut Gen) -> Call {
    let size = |g: &mut Gen| {
        let any = word(g);
        g.pick(&[0, 1, 40, 320, 5000, any])
    };
    match g.below(12) {
        0 => Call::Present(size(g), size(g)),
        1 => Call::Resize(size(g), size(g)),
        2 => Call::Title,
        3 => Call::Cursor,
        4 => Call::Fullscreen(g.chance(0.5)),
        5 => Call::Copy,
        6 => Call::Paste,
        7 => Call::Raise,
        8 => Call::Close,
        _ => Call::Poll,
    }
}

fn generate(g: &mut Gen) -> Run {
    let mut globals = vec![("wl_seat", 8), ("wl_data_device_manager", 3)];
    for optional in [
        ("zxdg_decoration_manager_v1", 1),
        ("wp_viewporter", 1),
        ("wp_fractional_scale_manager_v1", 1),
        ("wp_cursor_shape_manager_v1", 1),
        ("xdg_activation_v1", 1),
    ] {
        if g.chance(0.5) {
            globals.push(optional);
        }
    }
    let options = Options {
        globals,
        output_scale: g.pick(&[1, 2, 0, -3, i32::MAX]),
        fractional: g.pick(&[120, 180, 0, u32::MAX]),
        decoration_mode: g.pick(&[1, 2, 0, 77]),
        title_bar: title_bar(g),
    };
    let steps = (0..g.len(10))
        .map(|_| ((0..g.len(6)).map(|_| action(g)).collect(), call(g)))
        .collect();
    Run {
        options,
        steps,
        hang_up: g.chance(0.3),
    }
}

/// The descriptor, opened.
fn open(descriptor: &Descriptor) -> OwnedFd {
    match descriptor {
        Descriptor::Pipe => OwnedFd::from(std::io::pipe().expect("a pipe").0),
        Descriptor::File(bytes) => {
            let file = crate::wayland::sys::memfd(c"fuzz").expect("a memfd");
            file.write_all_at(bytes, 0).expect("the file is written");
            OwnedFd::from(file)
        }
    }
}

/// Performs the compositor's actions.
fn act(fake: &Fake, actions: &[Action]) {
    let mut objects: Vec<u32> = fake.state().objects.keys().copied().collect();
    objects.sort_unstable();
    objects.push(1);
    for action in actions {
        match action {
            Action::Event {
                object,
                opcode,
                args,
                fd,
            } => {
                let id = objects
                    .get(*object)
                    .copied()
                    .unwrap_or(*object as u32 + 0xff00_0000);
                let mut event = message(id, *opcode, args);
                if let Some(descriptor) = fd {
                    event = event.fd(open(descriptor));
                }
                fake.send(event);
            }
            Action::Garbage(bytes) => fake.send_bytes(bytes),
        }
    }
}

/// Makes the application's call, ignoring errors.
fn perform(
    backend: &mut crate::wayland::WaylandBackend,
    window: crate::window::WindowId,
    call: Call,
) {
    let deadline = Instant::now() + Duration::from_millis(2);
    let _ = match call {
        Call::Poll => backend.poll(Some(deadline), &mut Vec::new(), Blocking::DIRECT),
        Call::Present(width, height) => {
            let size = PhysicalSize::new(width.min(2000), height.min(2000));
            OwnedFrame::filled(size, [9, 99, 199, 255]).and_then(|frame| {
                backend.present(window, &frame.as_frame(), PhysicalRect::from_size(size))
            })
        }
        Call::Resize(width, height) => {
            backend.set_inner_size(window, PhysicalSize::new(width, height).into())
        }
        Call::Title => backend.set_title(window, "fuzz \u{1F600}\0 title"),
        Call::Cursor => backend.set_cursor(window, CursorIcon::Text),
        Call::Fullscreen(on) => backend.set_fullscreen(window, on),
        Call::Copy => backend.set_clipboard_text(window, String::from("copied")),
        Call::Paste => backend.request_clipboard_text(window),
        Call::Raise => backend.raise_window(window),
        Call::Close => backend.destroy_window(window),
    };
}

/// Title-bar settings a desktop may have: GNOME's, others' and odd ones.
fn title_bar(g: &mut Gen) -> TitleBarSettings {
    const LAYOUTS: [&str; 5] = [
        "appmenu:close",
        ":minimize,maximize,close",
        "close,minimize,maximize:",
        ":",
        "maximize,close:close,minimize",
    ];
    const ACTIONS: [&str; 4] = ["toggle-maximize", "minimize", "menu", "none"];
    TitleBarSettings {
        layout: ButtonLayout::parse(g.pick(&LAYOUTS)),
        double_click: TitleBarAction::parse(g.pick(&ACTIONS)),
        middle_click: TitleBarAction::parse(g.pick(&ACTIONS)),
        right_click: TitleBarAction::parse(g.pick(&ACTIONS)),
    }
}

#[test]
fn backends_against_a_hostile_compositor() {
    fuzz("wayland compositor", 150, generate, |run| {
        let (mut backend, fake) = Fake::start(run.options.clone());
        let options = WindowOptions::new("fuzz").with_inner_size(PhysicalSize::new(64, 48));
        let Ok(window) = backend.create_window(&options) else {
            return;
        };
        for (actions, call) in &run.steps {
            act(&fake, actions);
            perform(&mut backend, window, *call);
            if backend
                .poll(Some(Instant::now()), &mut Vec::new(), Blocking::DIRECT)
                .is_err()
            {
                break;
            }
        }
        if run.hang_up {
            fake.hang_up();
            let _ = backend.poll(
                Some(Instant::now() + Duration::from_millis(5)),
                &mut Vec::new(),
                Blocking::DIRECT,
            );
        }
        let _ = backend.read_pixels(window);
    });
}
