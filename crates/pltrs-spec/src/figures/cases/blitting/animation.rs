//! Blitted animations (`FuncAnimation(blit=True)`) a few frames in, their
//! timer fired by hand.

use std::time::Duration;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation};
use pltrs::prelude::*;

use super::{blitting, shifted, wave_x};
use crate::figures::{CaseResult, FigureCase};

/// Fires the animation timer of `fig` `ticks` times (matplotlib's
/// `event_source._on_timer()`).
fn tick(fig: &Figure, ticks: usize) -> CaseResult {
    let (timer, _) = *fig.timers().first().ok_or("the animation did not start")?;
    for _ in 0..ticks {
        fig.fire_timer(timer);
    }
    Ok(())
}

/// `simple_anim`, five frames in. The figure is drawn as the animation is
/// made, before the line becomes animated: the background saved at the
/// first frame still shows the wave as it was.
fn func_animation(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = wave_x();
    let line = ax.plot(x.clone(), shifted(&x, 0.0, f64::sin))?;
    let options = AnimationOptions {
        interval: Duration::from_millis(20),
        blit: true,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(fig, Frames::Endless, options, move |i| {
        line.set_data(&x, shifted(&x, i as f64 / 50.0, f64::sin));
        line.clone()
    });
    tick(fig, 5)?;
    drop(animation);
    Ok(())
}

/// A wave and a frame counter set up by an init function, seven frames
/// in.
fn func_animation_init(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 2.0 * std::f64::consts::PI)?
        .set_ylim(-1.1, 1.1)?;
    let line = ax.plot_with(
        Vec::<f64>::new(),
        Vec::<f64>::new(),
        LineProps::default().linewidth(3.0),
    )?;
    let counter = ax.text(0.05, 0.9, "");
    counter.set_transform(ax.trans_axes());
    let changed = vec![line.raw().clone(), counter.raw().clone()];
    let (init_line, init_counter, init_changed) = (line.clone(), counter.clone(), changed.clone());
    let init = move || {
        init_line.set_data(Vec::<f64>::new(), Vec::<f64>::new());
        init_counter.set_text("");
        init_changed.clone()
    };
    let x = wave_x();
    let animate = move |i: usize| {
        line.set_data(&x, shifted(&x, i as f64 / 10.0, f64::sin));
        counter.set_text(format!("frame {i}"));
        changed.clone()
    };
    let options = AnimationOptions {
        blit: true,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::with_init(fig, Frames::Count(20), options, init, animate);
    tick(fig, 7)?;
    drop(animation);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    blitting("blitting/func_animation", func_animation),
    blitting("blitting/func_animation_init", func_animation_init),
];
