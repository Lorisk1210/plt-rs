//! matplotlib's "Animations using Matplotlib" tutorial in pltrs (Rust):
//! `FuncAnimation` redrawing and blitting, `ArtistAnimation`-style frame
//! lists, saving as GIF, animated PNG and WebP and as an HTML player,
//! and recording frames drawn by hand with a movie writer.
//!
//! ```text
//! cargo run -p pltrs-examples --example animation_guide   # writes animation_guide_*.{gif,png,webp,html} here
//! ```
//!
//! Videos (`.mp4`, ...) go through ffmpeg, as in matplotlib, when it is
//! installed: [`FuncAnimation::save`] picks the writer from the file
//! name.

use std::time::Duration;

use pltrs::animation::{
    AnimationOptions, Frames, FuncAnimation, HtmlOptions, MovieOptions, MovieWriter,
};
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs_core::Point;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn main() -> Outcome {
    func_animation()?;
    blitted()?;
    artist_frames()?;
    movie_writer()?;
    Ok(())
}

/// `FuncAnimation(fig, update, frames=40, interval=30)` updating a line
/// and a scatter plot, saved with `ani.save("....gif")` and
/// `ani.to_jshtml()`.
fn func_animation() -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 80.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let t = linspace(0.0, 3.0, 40);
    let g = -9.81;
    let height = move |v0: f64, t: f64| g * t * t / 2.0 + v0 * t;
    let line = ax.plot_with(
        &t[..1],
        [height(12.0, t[0])],
        LineProps::default().label("v0 = 12 m/s"),
    )?;
    let dots = ax.scatter(&t[..1], [height(5.0, t[0])])?;
    ax.set_xlim(0.0, 3.0)?.set_ylim(-4.0, 10.0)?;
    ax.set_xlabel("time [s]").set_ylabel("height [m]");
    ax.legend(LegendOptions::default())?;

    let options = AnimationOptions {
        interval: Duration::from_millis(30),
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(t.len()), options, move |frame| {
        let shown = &t[..=frame];
        line.set_data(shown, shown.iter().map(|&t| height(12.0, t)));
        dots.set_offsets(
            shown
                .iter()
                .map(|&t| Point::new(t, height(5.0, t)))
                .collect(),
        );
    });
    animation.save("animation_guide_func.gif")?;
    animation.save("animation_guide_func.png")?;
    animation.save("animation_guide_func.webp")?;
    std::fs::write(
        "animation_guide_func.html",
        animation.to_jshtml(HtmlOptions::default())?,
    )?;
    Ok(())
}

/// `FuncAnimation(..., blit=True)`: only the animated artists the update
/// returns are drawn again over a saved background.
fn blitted() -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 80.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 120);
    let wave = ax.plot(&x, x.iter().map(|x| x.sin()))?;
    wave.set_animated(true);
    let options = AnimationOptions {
        interval: Duration::from_millis(40),
        blit: true,
        ..AnimationOptions::default()
    };
    let moving = wave.clone();
    let animation = FuncAnimation::new(&fig, Frames::Count(30), options, move |frame| {
        let phase = frame as f64 / 30.0 * 2.0 * std::f64::consts::PI;
        moving.set_ydata(moving.xdata().iter().map(|x| (x + phase).sin()));
        moving.clone()
    });
    animation.save("animation_guide_blit.gif")?;
    Ok(())
}

/// `ArtistAnimation(fig, [[artists of frame 0], ...])`: every frame's
/// artists are made up front and only that frame's are shown.
fn artist_frames() -> Outcome {
    let fig = Figure::with_size(3.0, 3.0, 80.0);
    let ax = fig.subplot();
    let images: Vec<_> = (0..12)
        .map(|frame| {
            let shift = f64::from(frame) / 12.0 * 2.0 * std::f64::consts::PI;
            let field = Grid::from_fn(40, 40, |r, c| {
                f64::midpoint((r as f64 / 6.0 + shift).sin(), (c as f64 / 6.0).cos())
            });
            let image = ax.imshow(field, ImshowOptions::default())?;
            image.set_visible(frame == 0);
            Ok(image)
        })
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    let options = AnimationOptions {
        interval: Duration::from_millis(80),
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(images.len()), options, move |frame| {
        for (i, image) in images.iter().enumerate() {
            image.set_visible(i == frame);
        }
    });
    animation.save("animation_guide_artists.gif")?;
    Ok(())
}

/// `writer = PillowWriter(fps=10); with writer.saving(fig, path, dpi):
/// ...; writer.grab_frame()`: frames the program draws itself.
fn movie_writer() -> Outcome {
    let fig = Figure::with_size(3.0, 3.0, 80.0);
    let ax = fig.subplot();
    ax.set_xlim(-1.2, 1.2)?.set_ylim(-1.2, 1.2)?;
    let dot = ax.plot_fmt([1.0], [0.0], "o")?;
    let options = MovieOptions {
        fps: Some(10.0),
        ..MovieOptions::default()
    };
    let mut writer = MovieWriter::new(&fig, "animation_guide_writer.gif", Some(80.0), options)?;
    for step in 0..20 {
        let angle = f64::from(step) / 20.0 * 2.0 * std::f64::consts::PI;
        dot.set_data([angle.cos()], [angle.sin()]);
        writer.grab_frame()?;
    }
    writer.finish()?;
    Ok(())
}
