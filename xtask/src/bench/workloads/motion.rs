//! Animations: every frame drawn and encoded as PNG into the page of
//! matplotlib's JavaScript player.

use std::rc::Rc;
use std::time::Duration;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation};
use pltrs::prelude::*;

use super::{BuildAnimation, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] =
    &[Workload::animation("animation/wave_30_frames", "animation_30", wave).sized(6.0, 3.2)];

/// 30 frames of a travelling wave packet: one line, its data replaced
/// every frame.
fn wave() -> BuildAnimation {
    const FRAMES: usize = 30;
    let x = linspace(0.0, 10.0, 400);
    let packets: Rc<Vec<Vec<f64>>> = Rc::new(
        (0..FRAMES)
            .map(|frame| {
                let centre = 2.0 + 6.0 * frame as f64 / (FRAMES - 1) as f64;
                x.iter()
                    .map(|x| (-((x - centre).powi(2)) / 1.5).exp() * (4.0 * (x - centre)).cos())
                    .collect()
            })
            .collect(),
    );
    Box::new(move |fig| {
        let ax = fig.subplot();
        let wave = ax.plot_with(&x, &packets[0], LineProps::default().linewidth(2.0))?;
        ax.set_xlim(0.0, 10.0)?.set_ylim(-1.2, 1.2)?;
        ax.set_title("A travelling wave packet");
        let options = AnimationOptions {
            interval: Duration::from_millis(80),
            ..AnimationOptions::default()
        };
        let packets = Rc::clone(&packets);
        Ok(FuncAnimation::new(
            fig,
            Frames::Count(FRAMES),
            options,
            move |frame| {
                wave.set_ydata(packets[frame].clone());
                wave.clone()
            },
        ))
    })
}
