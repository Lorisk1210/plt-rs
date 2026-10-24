//! The first frame of the gallery's animations: the figure as it is
//! before the animation starts, or as its `init_func` leaves it, the
//! first frame of an `ArtistAnimation` (`dynamic_image` and the second
//! figure of the animations guide), which hides the artists of the others,
//! and the first pass of an animation drawn by a loop (`animation_demo`
//! and `frame_grabbing_sgskip`).

use std::f64::consts::PI;

use plt::axes::{AxisName, HistPlotOptions, ImshowOptions, Limits, ScatterColors, ScatterOptions};
use plt::coords::Coords;
use plt::figure::Share;
use plt::fonts::FontWeight;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::patches::{ConnectionEnd, ConnectionPatch};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::stats::{Bins, HistOptions};
use plt::text::{HAlign, VAlign};

use super::{SEED, uniform_rows};
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// An empty line with `props`.
fn empty_line(ax: &Axes, props: LineProps) -> CaseResult {
    ax.plot_with(Vec::<f64>::new(), Vec::<f64>::new(), props)?;
    Ok(())
}

fn animate_decay(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    empty_line(&ax, LineProps::default().linewidth(2.0))?;
    ax.grid(true);
    ax.set_ylim(-1.1, 1.1)?.set_xlim(0.0, 1.0)?;
    Ok(())
}

fn animated_histogram(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges(linspace(-4.0, 4.0, 100))),
            ..HistOptions::default()
        },
        props: PatchProps::default()
            .linewidth(1.0)
            .edgecolor("yellow")
            .facecolor("green")
            .alpha(0.5),
        ..HistPlotOptions::default()
    };
    ax.hist(&[SpecRng::new(SEED).normal(1000, 0.0, 1.0)], options)?;
    ax.set_limits_with(
        AxisName::Y,
        Limits {
            high: Some(55.0),
            ..Limits::default()
        },
    )?;
    Ok(())
}

fn animation_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let frame = uniform_rows(&mut SpecRng::new(SEED), 50, 50);
    ax.imshow(frame, ImshowOptions::default())?;
    ax.set_title("frame 0");
    Ok(())
}

fn bayes_update(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    empty_line(&ax, LineProps::fmt("k-")?)?;
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 10.0)?;
    ax.grid(true);
    ax.axvline(
        0.7,
        LineProps::default()
            .linestyle(LineStyle::Dashed)
            .color("black"),
    );
    Ok(())
}

fn double_pendulum(fig: &Figure) -> CaseResult {
    let length = 2.0;
    let ax = fig.subplot();
    ax.set_autoscale_on(false);
    ax.set_xlim(-length, length)?.set_ylim(-length, 1.0)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.grid(true);
    empty_line(&ax, LineProps::fmt("o-")?.linewidth(2.0))?;
    empty_line(&ax, LineProps::fmt(".-")?.linewidth(1.0).markersize(2.0))?;
    ax.text(0.05, 0.9, "").set_transform(ax.trans_axes());
    Ok(())
}

fn frame_grabbing_sgskip(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let mut step = || 0.1 * g.normal(1, 0.0, 1.0)[0];
    let (x0, y0) = (step(), step());
    let ax = fig.subplot();
    ax.plot_fmt([x0], [y0], "k-o")?;
    ax.set_xlim(-5.0, 5.0)?.set_ylim(-5.0, 5.0)?;
    Ok(())
}

fn dynamic_image(fig: &Figure) -> CaseResult {
    let x = map(&linspace(0.0, 2.0 * PI, 120), |x| x + PI / 15.0);
    let y = map(&linspace(0.0, 2.0 * PI, 100), |y| y + PI / 30.0);
    let image = Grid::from_fn(y.len(), x.len(), |r, c| x[c].sin() + y[r].cos());
    fig.subplot().imshow(image, ImshowOptions::default())?;
    Ok(())
}

fn multiple_axes(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        1,
        2,
        Some(vec![1.0, 3.0]),
        None,
        SubplotOverrides {
            wspace: Some(0.0),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::All)?;
    let (axl, axr) = (&axs[(0, 0)], &axs[(0, 1)]);
    axl.set_aspect(Aspect::Ratio(1.0))?;
    axr.set_box_aspect(Some(1.0 / 3.0))?;
    axr.yaxis().set_visible(false);
    axr.xaxis().set_ticks([0.0, PI, 2.0 * PI]);
    axr.xaxis().set_ticklabels(["0", r"$\pi$", r"$2\pi$"])?;
    let x = linspace(0.0, 2.0 * PI, 50);
    axl.plot_with(
        map(&x, f64::cos),
        map(&x, f64::sin),
        LineProps::fmt("k")?.linewidth(0.3),
    )?;
    axl.plot_fmt([0.0], [0.0], "o")?;
    axr.plot(&x, map(&x, f64::sin))?;
    let con = ConnectionPatch::new(
        ConnectionEnd::new(
            (1.0, 0.0),
            Coords::Transform(axl.trans_data()),
            Some(axl.id()),
        ),
        ConnectionEnd::new(
            (0.0, 0.0),
            Coords::Transform(axr.trans_data()),
            Some(axr.id()),
        ),
    );
    fig.add_artist(con)
        .set_color("C0")
        .set_linestyle(LineStyle::Dotted);
    Ok(())
}

fn pause_resume(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("Click to pause/resume the animation");
    let x = linspace(-0.1, 0.1, 1000);
    let peak = 1.0 / (4.0 * PI * 2e-4 * 0.1).powf(0.5);
    ax.plot(
        &x,
        map(&x, |x| peak * (-(x * x) / (4.0 * 2e-4 * 0.1)).exp()),
    )?;
    Ok(())
}

fn random_walk(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let empty: [f64; 0] = [];
    for _ in 0..40 {
        ax.plot(&empty, &empty, &empty)?;
    }
    ax.set_xlim(0.0, 1.0)?.set_xlabel("X");
    ax.set_ylim(0.0, 1.0)?.set_ylabel("Y");
    ax.set_zlim(0.0, 1.0)?.set_zlabel("Z");
    Ok(())
}

fn simple_anim(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 2.0 * PI, 0.01);
    fig.subplot().plot(&x, map(&x, f64::sin))?;
    Ok(())
}

fn simple_scatter(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)?;
    ax.scatter([1.0], [0.0])?;
    Ok(())
}

fn strip_chart(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_line(&[0.0], &[0.0], LineProps::default())?;
    ax.set_ylim(-0.1, 1.1)?.set_xlim(0.0, 2.0)?;
    Ok(())
}

fn unchained(fig: &Figure) -> CaseResult {
    fig.set_facecolor(Color::from("black").resolve()?);
    let ax = fig.subplot();
    ax.set_frame_on(false);
    let data = uniform_rows(&mut SpecRng::new(SEED), 64, 75);
    let x = linspace(-1.0, 1.0, data.cols());
    let gauss = map(&x, |x| 1.5 * (-4.0 * (x * x)).exp());
    for (i, row) in data.iter_rows().enumerate() {
        let i = i as f64;
        let xscale = 1.0 - i / 200.0;
        let lw = 1.5 - i / 100.0;
        let y = row.iter().zip(&gauss).map(|(d, g)| i + g * d);
        ax.plot_with(
            map(&x, |x| xscale * x),
            y.collect::<Vec<f64>>(),
            LineProps::default().color("w").linewidth(lw),
        )?;
    }
    ax.set_ylim(-1.0, 70.0)?;
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    for (text, halign, weight) in [
        ("MATPLOTLIB ", HAlign::Right, "light"),
        ("UNCHAINED", HAlign::Left, "bold"),
    ] {
        ax.text(0.5, 1.0, text)
            .set_family(["sans-serif"])
            .set_weight(weight.parse::<FontWeight>()?)
            .set_fontsize(16.0)
            .set_halign(halign)
            .set_valign(VAlign::Bottom)
            .set_color("w")
            .set_transform(ax.trans_axes());
    }
    Ok(())
}

fn animations_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (v0, v02) = (12.0, 5.0);
    ax.scatter_with(
        [0.0],
        [0.0],
        ScatterOptions {
            colors: ScatterColors::Single("b".into()),
            sizes: Some(vec![5.0]),
            ..ScatterOptions::default()
        },
    )?
    .set_label(format!("v0 = {v0} m/s"));
    ax.plot_with(
        [0.0],
        [0.0],
        LineProps::default().label(format!("v0 = {v02} m/s")),
    )?;
    ax.set_xlim(0.0, 3.0)?.set_ylim(-4.0, 10.0)?;
    ax.set_xlabel("Time [s]").set_ylabel("Z [m]");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn animations_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut g = SpecRng::new(SEED);
    let mut data = [20.0; 4];
    let colors = ["tab:blue", "tab:red", "tab:green", "tab:purple"].map(Color::from);
    for frame in 0..20 {
        for (value, step) in data.iter_mut().zip(g.integers(4, 0, 10)) {
            *value += step as f64;
        }
        let container = ax.barh(
            [1.0, 2.0, 3.0, 4.0],
            data,
            BarOptions {
                color: colors.to_vec(),
                ..BarOptions::default()
            },
        )?;
        if frame > 0 {
            for bar in &container.patches {
                bar.set_visible(false);
            }
        }
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/animate_decay", animate_decay),
    FigureCase::new("gallery_tutorials/animated_histogram", animated_histogram),
    FigureCase::new("gallery_tutorials/animation_demo", animation_demo),
    FigureCase::new("gallery_tutorials/bayes_update", bayes_update),
    FigureCase::new("gallery_tutorials/double_pendulum", double_pendulum).sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_tutorials/frame_grabbing_sgskip",
        frame_grabbing_sgskip,
    ),
    FigureCase::new("gallery_tutorials/dynamic_image", dynamic_image),
    FigureCase::new("gallery_tutorials/multiple_axes", multiple_axes).sized(6.0, 2.0, 100.0),
    FigureCase::new("gallery_tutorials/pause_resume", pause_resume),
    FigureCase::new("gallery_tutorials/random_walk", random_walk),
    FigureCase::new("gallery_tutorials/simple_anim", simple_anim),
    FigureCase::new("gallery_tutorials/simple_scatter", simple_scatter),
    FigureCase::new("gallery_tutorials/strip_chart", strip_chart),
    FigureCase::new("gallery_tutorials/unchained", unchained).sized(8.0, 8.0, 100.0),
    FigureCase::new("gallery_tutorials/animations_1", animations_1),
    FigureCase::new("gallery_tutorials/animations_2", animations_2),
];
