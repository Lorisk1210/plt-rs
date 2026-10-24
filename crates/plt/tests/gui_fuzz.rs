//! The figure windows survive any input: thousands of pseudo-random mouse
//! moves, clicks, drags, wheel turns, key presses, resizes and scale
//! changes reach headless windows showing rectilinear, logarithmic,
//! image, polar and 3-D axes, and whatever dialogs those events open.
//! Nothing may panic.

use std::time::Duration;

use plt::axes::ImshowOptions;
use plt::gui::Session;
use plt::numeric::Grid;
use plt::prelude::*;
use plt_core::rng::SplitMix64;
use plt_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize,
    ScrollDelta, WindowEvent, WindowId,
};

/// A deterministic stream of pseudo-random numbers (`SplitMix64`).
struct Stream(SplitMix64);

impl Stream {
    /// A number in `0..n`.
    fn below(&mut self, n: usize) -> usize {
        self.0.below(n)
    }

    /// A number in `[low, high)`.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.0.next_f64()
    }

    /// One of `items`.
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

/// The figures under test: four kinds of 2-D axes, and 3-D axes.
fn figures() -> Result<Vec<(Figure, String)>, Box<dyn std::error::Error>> {
    let flat = Figure::with_size(4.0, 3.0, 50.0);
    let axs = flat.subplots(2, 2)?;
    axs[(0, 0)].plot(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0])?;
    axs[(0, 1)].scatter(vec![1.0, 10.0, 100.0], vec![1.0, 5.0, 50.0])?;
    axs[(0, 1)].set_xscale("log")?;
    axs[(0, 1)].set_yscale("log")?;
    let image = Grid::from_fn(8, 8, |i, j| (i * j) as f64);
    axs[(1, 0)].imshow(image, ImshowOptions::default())?;
    axs[(1, 1)].bar(vec![0.0, 1.0, 2.0], [3.0, 1.0, 2.0], BarOptions::default())?;
    let round = Figure::with_size(3.0, 3.0, 50.0);
    round
        .polar_subplot()
        .plot(vec![0.0, 1.0, 2.0, 3.0], vec![1.0, 2.0, 1.5, 3.0])?;
    let deep = Figure::with_size(3.0, 3.0, 50.0);
    let ax3 = deep.subplot_3d();
    ax3.scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5], &[1.0, 0.0, 2.0])?;
    ax3.plot(&[0.0, 1.0, 2.0], &[2.0, 1.0, 0.0], &[0.0, 1.0, 1.0])?;
    Ok(vec![
        (flat, "Flat".to_owned()),
        (round, "Polar".to_owned()),
        (deep, "3D".to_owned()),
    ])
}

/// The keys tried: matplotlib's default key bindings and their
/// neighbours, editing and navigation keys.
const KEYS: [Key; 34] = [
    Key::Character('f'),
    Key::Character('g'),
    Key::Character('G'),
    Key::Character('h'),
    Key::Character('r'),
    Key::Character('k'),
    Key::Character('l'),
    Key::Character('L'),
    Key::Character('o'),
    Key::Character('p'),
    Key::Character('s'),
    Key::Character('x'),
    Key::Character('y'),
    Key::Character('c'),
    Key::Character('v'),
    Key::Character('a'),
    Key::Character('0'),
    Key::Character('1'),
    Key::Character('.'),
    Key::Character('-'),
    Key::Character(' '),
    Key::Escape,
    Key::Enter,
    Key::Backspace,
    Key::Delete,
    Key::Tab,
    Key::Left,
    Key::Right,
    Key::Up,
    Key::Down,
    Key::Home,
    Key::End,
    Key::F(1),
    Key::Control,
];

/// A pseudo-random event for a window of `size` physical pixels.
fn event(stream: &mut Stream, size: PhysicalSize) -> WindowEvent {
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    let position = PhysicalPosition::new(
        stream.between(-20.0, width + 20.0),
        stream.between(-20.0, height + 20.0),
    );
    let modifiers = stream.pick(&[
        Modifiers::NONE,
        Modifiers::NONE,
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::CONTROL,
        Modifiers::ALT,
    ]);
    let state = stream.pick(&[ButtonState::Pressed, ButtonState::Released]);
    match stream.below(100) {
        0..35 => WindowEvent::CursorMoved {
            position,
            modifiers,
        },
        35..60 => WindowEvent::MouseInput {
            button: stream.pick(&[MouseButton::Left, MouseButton::Middle, MouseButton::Right]),
            state,
            position,
            modifiers,
            click_count: stream.pick(&[1, 1, 2]),
        },
        60..68 => WindowEvent::MouseWheel {
            delta: ScrollDelta::Lines {
                x: 0.0,
                y: stream.pick(&[-3.0, -1.0, 1.0, 2.0]),
            },
            position,
            modifiers,
        },
        68..94 => {
            let key = stream.pick(&KEYS);
            let text = match key {
                Key::Character(c) => Some(c.to_string()),
                _ => None,
            };
            WindowEvent::Key(KeyEvent {
                key,
                state,
                repeat: false,
                text,
                modifiers,
                scancode: 0,
            })
        }
        94..97 => WindowEvent::Resized(PhysicalSize::new(
            stream.pick(&[1, 40, 200, 640]),
            stream.pick(&[1, 30, 150, 480]),
        )),
        97..99 => WindowEvent::ScaleFactorChanged(stream.pick(&[0.5, 1.0, 2.0, 3.0])),
        _ => WindowEvent::Focused(stream.pick(&[true, false])),
    }
}

#[test]
fn windows_survive_random_input() -> Result<(), Box<dyn std::error::Error>> {
    let figures = figures()?;
    let (mut session, handle) = Session::headless();
    session.show(&figures)?;
    let mut stream = Stream(SplitMix64::new(20_261_019));
    for _ in 0..400 {
        let windows: Vec<WindowId> = handle.windows();
        if windows.is_empty() {
            session.show(&figures)?;
            continue;
        }
        for _ in 0..8 {
            let window = stream.pick(&windows);
            let Some(snapshot) = handle.window(window) else {
                continue;
            };
            // A window closed by an earlier event refuses input.
            let _ = handle.inject(window, event(&mut stream, snapshot.size));
        }
        // Errors (a dialog's file name, say) are fine; panics are not.
        let _ = session.pause(Duration::from_millis(1));
    }
    Ok(())
}
