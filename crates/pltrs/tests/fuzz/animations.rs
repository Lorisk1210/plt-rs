//! Animations: figures animated by frame functions that change them in
//! every frame — new data (empty, of unequal lengths, NaN, huge), view
//! limits, the figure's size and resolution (so that frames differ in
//! size), texts added, the line hidden or removed — with any number of
//! frames and any interval, blitted or not, saved as animated PNG, GIF
//! and WebP and as the HTML player, with PNG or SVG frames under any
//! embed limit. A save may fail, but never panic.

use std::num::NonZeroU32;
use std::time::Duration;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation, HtmlOptions, PlayMode};
use pltrs::artist::ArtistHandle;
use pltrs::figure::Figure;

use crate::harness::{Gen, fuzz};

/// What a frame changes.
#[derive(Clone, Debug)]
enum Change {
    Data(Vec<f64>, Vec<f64>),
    Limits(f64, f64),
    Size(f64, f64),
    Dpi(f64),
    Text(f64, f64, String),
    Visible(bool),
    Remove,
}

/// How the animation is saved.
#[derive(Clone, Copy, Debug)]
enum Output {
    Apng,
    Gif,
    Webp,
    Html {
        fps: u32,
        mode: Option<PlayMode>,
        svg_frames: bool,
        embed_limit: f64,
    },
}

/// An animation and how it is saved.
#[derive(Debug)]
struct Plan {
    size: (f64, f64),
    dpi: f64,
    frames: usize,
    interval: Duration,
    blit: bool,
    changes: Vec<Change>,
    output: Output,
}

/// A side of the figure in inches: small mostly, so that frames render
/// quickly, now and then any number.
fn side(g: &mut Gen) -> f64 {
    if g.chance(0.05) {
        g.number()
    } else {
        g.between(0.2, 2.0)
    }
}

impl Change {
    fn generate(g: &mut Gen) -> Self {
        match g.below(10) {
            0..3 => {
                let len = g.len(12);
                let other = if g.chance(0.1) { g.len(12) } else { len };
                Self::Data(g.numbers(len), g.numbers(other))
            }
            3 => Self::Limits(g.number(), g.number()),
            4 => Self::Size(side(g), side(g)),
            5 => Self::Dpi(if g.chance(0.2) {
                g.number()
            } else {
                g.between(10.0, 60.0)
            }),
            6 => Self::Text(
                g.number_in(0.0, 1.0),
                g.number_in(0.0, 1.0),
                g.soup(&["x", "$x^2$", "\n", " "], 4),
            ),
            7 | 8 => Self::Visible(g.chance(0.5)),
            _ => Self::Remove,
        }
    }
}

impl Output {
    fn generate(g: &mut Gen) -> Self {
        match g.below(4) {
            0 => Self::Apng,
            1 => Self::Gif,
            2 => Self::Webp,
            _ => Self::Html {
                fps: g.pick(&[0, 1, 5, 30, u32::MAX]),
                mode: g.pick(&[
                    None,
                    Some(PlayMode::Once),
                    Some(PlayMode::Loop),
                    Some(PlayMode::Reflect),
                ]),
                svg_frames: g.chance(0.3),
                embed_limit: g.pick(&[20.0, 0.0, 1e-4, -1.0, f64::NAN]),
            },
        }
    }
}

impl Plan {
    fn generate(g: &mut Gen) -> Self {
        Self {
            size: (side(g), side(g)),
            dpi: g.between(10.0, 50.0),
            frames: g.len(6),
            interval: Duration::from_millis(g.pick(&[0, 1, 40, 200, 10_000, u64::MAX >> 20])),
            blit: g.chance(0.3),
            changes: (0..=g.len(5)).map(|_| Change::generate(g)).collect(),
            output: Output::generate(g),
        }
    }
}

/// Animates a figure of one line with the plan's frame function and saves
/// the animation as the plan says.
fn animate(plan: &Plan) {
    let fig = Figure::with_size(plan.size.0, plan.size.1, plan.dpi);
    let ax = fig.subplot();
    let Ok(line) = ax.plot(vec![0.0, 1.0, 2.0], vec![1.0, 0.0, 2.0]) else {
        return;
    };
    let options = AnimationOptions {
        interval: plan.interval,
        blit: plan.blit,
        ..AnimationOptions::default()
    };
    let changes = plan.changes.clone();
    let (frame_fig, frame_ax, frame_line) = (fig.clone(), ax.clone(), line.clone());
    let animation = FuncAnimation::new(&fig, Frames::Count(plan.frames), options, move |frame| {
        match &changes[frame % changes.len()] {
            Change::Data(x, y) => {
                frame_line.set_data(x.clone(), y.clone());
            }
            Change::Limits(left, right) => {
                let _ = frame_ax.set_xlim(*left, *right);
            }
            Change::Size(width, height) => {
                let _ = frame_fig.set_size_inches(*width, *height);
            }
            Change::Dpi(dpi) => {
                frame_fig.set_dpi(*dpi);
            }
            Change::Text(x, y, text) => {
                frame_ax.text(*x, *y, text.clone());
            }
            Change::Visible(visible) => {
                frame_line.set_visible(*visible);
            }
            Change::Remove => frame_line.remove(),
        }
        frame_line.clone()
    });
    match plan.output {
        Output::Apng => {
            let _ = animation.to_apng();
        }
        Output::Gif => {
            let _ = animation.to_gif();
        }
        Output::Webp => {
            let _ = animation.to_webp();
        }
        Output::Html {
            fps,
            mode,
            svg_frames,
            embed_limit,
        } => pltrs::rc::context(|| {
            let _ = pltrs::rc::set("animation.embed_limit", &embed_limit.to_string());
            if svg_frames {
                let _ = pltrs::rc::set("animation.frame_format", "svg");
            }
            let options = HtmlOptions {
                fps: NonZeroU32::new(fps),
                default_mode: mode,
            };
            let _ = animation.to_jshtml(options);
        }),
    }
}

#[test]
fn animations_changing_their_figure() {
    fuzz("animations", 150, Plan::generate, animate);
}
