//! `figures/cases_lines.py`.

use std::f64::consts::TAU;

use pltrs::axes::{AxisName, StairsOptions};
use pltrs::lines::{AxLineThrough, DrawStyle, LineProps, LineStyle, MarkEvery, StepWhere};
use pltrs::markers::{FillStyle, Marker, PolygonStyle};
use pltrs::numeric::{arange, linspace, logspace};
use pltrs::orientation::Orientation;
use pltrs::patches::StepBaseline;
use pltrs::prelude::ArtistHandle;
use pltrs::scale::LogScale;
use pltrs::transforms::Point;
use pltrs_render::{CapStyle, JoinStyle};

use crate::figures::FigureCase;
use crate::figures::cases::common::map;
use crate::rng::SpecRng;

/// `X = np.linspace(0, 2 * np.pi, 50)`.
fn big_x() -> Vec<f64> {
    linspace(0.0, TAU, 50)
}

fn props() -> LineProps {
    LineProps::default()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("lines/y_only", |fig| {
        fig.subplot().plot_y(map(&big_x(), f64::sin));
        Ok(())
    }),
    FigureCase::new("lines/xy_two_series", |fig| {
        let ax = fig.subplot();
        let x = big_x();
        ax.plot(&x, map(&x, f64::sin))?;
        ax.plot(&x, map(&x, f64::cos))?;
        Ok(())
    }),
    FigureCase::new("lines/color_cycle_wraps", |fig| {
        let ax = fig.subplot();
        for i in 0..12 {
            let i = f64::from(i);
            ax.plot_with([0.0, 1.0], [i, i + 0.5], props().linewidth(3.0))?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/columns_of_2d_array", |fig| {
        let ax = fig.subplot();
        let t = linspace(0.0, 1.0, 20);
        for power in 1..=3 {
            ax.plot(&t, map(&t, |v| v.powi(power)))?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/multiple_pairs_one_call", |fig| {
        let ax = fig.subplot();
        let t = linspace(0.0, 2.0, 30);
        ax.plot_fmt(&t, &t, "r--")?;
        ax.plot_fmt(&t, map(&t, |v| v.powi(2)), "bs")?;
        ax.plot_fmt(&t, map(&t, |v| v.powf(1.5)), "g^")?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/fmt_strings", |fig| {
        let ax = fig.subplot();
        let t = arange(0.0, 10.0, 1.0);
        let formats = ["ro--", "g^", "b-.", "k:", "cs-", "m+", "yD:", "C7x-"];
        for (i, format) in formats.into_iter().enumerate() {
            let shift = 3.0 * i as f64;
            ax.plot_fmt(&t, map(&t, |v| v + shift), format)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/linestyles_named", |fig| {
        let ax = fig.subplot();
        let styles = ["-", "--", "-.", ":", "solid", "dashed", "dashdot", "dotted"];
        for (i, style) in styles.into_iter().enumerate() {
            let y = i as f64;
            ax.plot_with(
                [0.0, 1.0],
                [y, y],
                props().linestyle(style.parse::<LineStyle>()?),
            )?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/linestyles_dash_tuples", |fig| {
        let ax = fig.subplot();
        let styles: [(f64, &[f64]); 5] = [
            (0.0, &[1.0, 1.0]),
            (0.0, &[5.0, 2.0]),
            (5.0, &[10.0, 3.0]),
            (0.0, &[3.0, 5.0, 1.0, 5.0, 1.0, 5.0]),
            (0.0, &[8.0, 2.0, 1.0, 2.0]),
        ];
        for (i, (offset, pattern)) in styles.into_iter().enumerate() {
            let y = i as f64;
            let style = LineStyle::dashes(offset, pattern.to_vec());
            ax.plot_with([0.0, 1.0], [y, y], props().linestyle(style).color("k"))?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/set_dashes_and_gapcolor", |fig| {
        let ax = fig.subplot();
        let dashes = |pattern: &[f64]| LineStyle::dashes(0.0, pattern.to_vec());
        ax.plot_with(
            [0.0, 1.0],
            [0.0, 0.0],
            props().linestyle(dashes(&[6.0, 2.0])),
        )?;
        let gapped = props()
            .linestyle(LineStyle::Dashed)
            .gapcolor("tab:red")
            .linewidth(3.0);
        ax.plot_with([0.0, 1.0], [1.0, 1.0], gapped)?;
        let gapped = props()
            .linestyle(dashes(&[2.0, 2.0, 10.0, 2.0]))
            .gapcolor("0.8")
            .linewidth(3.0);
        ax.plot_with([0.0, 1.0], [2.0, 2.0], gapped)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/linewidths", |fig| {
        let ax = fig.subplot();
        for (i, width) in [0.25, 0.5, 1.0, 2.0, 4.0, 8.0].into_iter().enumerate() {
            let y = i as f64;
            ax.plot_with([0.0, 1.0], [y, y + 1.0], props().linewidth(width))?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/dashes_scale_with_linewidth", |fig| {
        let ax = fig.subplot();
        for (i, width) in [0.5, 1.0, 2.0, 4.0].into_iter().enumerate() {
            let y = i as f64;
            ax.plot_with(
                [0.0, 1.0],
                [y, y],
                LineProps::fmt("--")?.linewidth(width).color("k"),
            )?;
            let dotted = LineProps::fmt(":")?.linewidth(width).color("b");
            ax.plot_with([0.0, 1.0], [y + 0.4, y + 0.4], dotted)?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/solid_capstyles", |fig| {
        let ax = fig.subplot();
        for (i, cap) in [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting]
            .into_iter()
            .enumerate()
        {
            let y = i as f64;
            let style = props().linewidth(14.0).solid_capstyle(cap);
            ax.plot_with([0.2, 0.8], [y, y], style)?;
        }
        ax.set_xlim(0.0, 1.0)?;
        ax.set_ylim(-1.0, 3.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/dash_capstyles", |fig| {
        let ax = fig.subplot();
        for (i, cap) in [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting]
            .into_iter()
            .enumerate()
        {
            let y = i as f64;
            let style = LineProps::fmt("--")?.linewidth(8.0).dash_capstyle(cap);
            ax.plot_with([0.0, 1.0], [y, y], style)?;
        }
        ax.set_ylim(-1.0, 3.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/joinstyles", |fig| {
        let ax = fig.subplot();
        for (i, join) in [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel]
            .into_iter()
            .enumerate()
        {
            let x0 = 3.0 * i as f64;
            let style = props().linewidth(12.0).solid_joinstyle(join);
            ax.plot_with([x0, x0 + 1.0, x0 + 2.0], [0.5, 1.5, 0.5], style)?;
        }
        ax.set_xlim(-1.0, 9.0)?;
        ax.set_ylim(0.0, 2.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/markers_filled", |fig| {
        let ax = fig.subplot();
        let markers = [
            "o", "v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X",
        ];
        for (i, marker) in markers.into_iter().enumerate() {
            let y = i as f64;
            let style = props()
                .marker(marker.parse::<Marker>()?)
                .markersize(10.0)
                .linestyle(LineStyle::None);
            ax.plot_with([0.0, 1.0, 2.0], [y; 3], style)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/markers_unfilled", |fig| {
        let ax = fig.subplot();
        let named = [".", ",", "1", "2", "3", "4", "+", "x", "|", "_"];
        let markers = named
            .into_iter()
            .map(str::parse::<Marker>)
            .chain((0..12).map(Marker::from_code));
        for (i, marker) in markers.enumerate() {
            let style = props()
                .marker(marker?)
                .markersize(12.0)
                .markeredgewidth(1.5)
                .color("k");
            ax.plot_with([(i % 11) as f64], [(i / 11) as f64], style)?;
        }
        ax.set_xlim(-1.0, 11.0)?;
        ax.set_ylim(-1.0, 2.0)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/markers_mathtext", |fig| {
        let ax = fig.subplot();
        let markers = [r"$\clubsuit$", r"$f$", r"$\alpha$", r"$\heartsuit$"];
        for (i, marker) in markers.into_iter().enumerate() {
            let y = i as f64;
            let style = props().marker(marker.parse::<Marker>()?).markersize(15.0);
            ax.plot_with([0.0, 1.0, 2.0], [y, y + 0.5, y], style)?;
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/markers_polygon_tuples_and_vertices", |fig| {
        let ax = fig.subplot();
        let polygon = |sides, style, angle| Marker::Polygon {
            sides,
            style,
            angle,
        };
        let markers = [
            polygon(5, PolygonStyle::Regular, 0.0),
            polygon(5, PolygonStyle::Star, 0.0),
            polygon(5, PolygonStyle::Asterisk, 0.0),
            polygon(6, PolygonStyle::Regular, 30.0),
            polygon(3, PolygonStyle::Star, 90.0),
            Marker::Vertices(vec![
                Point::new(-1.0, -1.0),
                Point::new(1.0, -1.0),
                Point::new(0.0, 1.5),
            ]),
        ];
        for (i, marker) in markers.into_iter().enumerate() {
            let style = props()
                .marker(marker)
                .markersize(18.0)
                .linestyle(LineStyle::None);
            ax.plot_with([i as f64], [0.0], style)?;
        }
        ax.set_xlim(-1.0, 6.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/marker_size_edge_face", |fig| {
        let ax = fig.subplot();
        let t = arange(0.0, 6.0, 1.0);
        let shifted = |dy: f64| map(&t, |v| v + dy);
        ax.plot_with(&t, &t, LineProps::fmt("o-")?.markersize(4.0))?;
        let styled = LineProps::fmt("o-")?
            .markersize(12.0)
            .markerfacecolor("yellow")
            .markeredgecolor("red")
            .markeredgewidth(2.0);
        ax.plot_with(&t, shifted(2.0), styled)?;
        let hollow = LineProps::fmt("s--")?
            .markersize(9.0)
            .markerfacecolor("none")
            .markeredgecolor("k");
        ax.plot_with(&t, shifted(4.0), hollow)?;
        let edgeless = LineProps::fmt("D")?.markersize(8.0).markeredgewidth(0.0);
        ax.plot_with(&t, shifted(6.0), edgeless)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/fillstyles", |fig| {
        let ax = fig.subplot();
        let styles = [
            FillStyle::Full,
            FillStyle::Left,
            FillStyle::Right,
            FillStyle::Bottom,
            FillStyle::Top,
            FillStyle::None,
        ];
        for (i, fill) in styles.into_iter().enumerate() {
            let x = i as f64;
            let circles = LineProps::fmt("o")?
                .markersize(20.0)
                .fillstyle(fill)
                .markerfacecoloralt("tab:red")
                .color("tab:blue");
            ax.plot_with([x], [0.0], circles)?;
            let diamonds = LineProps::fmt("D")?
                .markersize(20.0)
                .fillstyle(fill)
                .markerfacecoloralt("0.8")
                .color("k");
            ax.plot_with([x], [1.0], diamonds)?;
        }
        ax.set_xlim(-1.0, 6.0)?;
        ax.set_ylim(-1.0, 2.0)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("lines/markevery_variants", |fig| {
        let ax = fig.subplot();
        let t = linspace(0.0, 10.0, 101);
        let variants = [
            MarkEvery::Every { start: 0, step: 5 },
            MarkEvery::Every { start: 3, step: 10 },
            MarkEvery::Indices(vec![0, 25, 50, 75, 100]),
            MarkEvery::Slice {
                start: Some(10),
                stop: Some(90),
                step: Some(20),
            },
            MarkEvery::Distance {
                start: 0.0,
                step: 0.2,
            },
        ];
        for (i, every) in variants.into_iter().enumerate() {
            let shift = 2.5 * i as f64;
            let style = LineProps::fmt("o-")?.markevery(Some(every)).markersize(5.0);
            ax.plot_with(&t, map(&t, |v| v.sin() + shift), style)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/drawstyles", |fig| {
        let ax = fig.subplot();
        let t = arange(0.0, 8.0, 1.0);
        let y = [1.0, 3.0, 2.0, 4.0, 3.0, 5.0, 4.0, 6.0];
        let styles = [
            DrawStyle::Default,
            DrawStyle::StepsPre,
            DrawStyle::StepsMid,
            DrawStyle::StepsPost,
        ];
        for (i, style) in styles.into_iter().enumerate() {
            let shift = 6.0 * i as f64;
            let line = LineProps::fmt("o-")?.drawstyle(style);
            ax.plot_with(&t, map(&y, |v| v + shift), line)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/alpha_overlap", |fig| {
        let ax = fig.subplot();
        let t = [0.0, 1.0];
        for i in 0..5 {
            let shift = 0.1 * f64::from(i);
            let style = props().linewidth(20.0).alpha(0.3).color("tab:blue");
            ax.plot_with(t, map(&t, |v| v + shift), style)?;
        }
        let style = props().linewidth(20.0).color("tab:red").alpha(0.6);
        ax.plot_with(t, map(&t, |v| 1.0 - v), style)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/antialiased_off", |fig| {
        let ax = fig.subplot();
        let x = big_x();
        let sine = props().antialiased(false).linewidth(2.0);
        ax.plot_with(&x, map(&x, f64::sin), sine)?;
        ax.plot_with(
            &x,
            map(&x, f64::cos),
            LineProps::fmt("o")?.antialiased(false),
        )?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/zorder_overrides_call_order", |fig| {
        let ax = fig.subplot();
        let thick = |zorder| props().linewidth(15.0).zorder(zorder);
        ax.plot_with([0.0, 1.0], [0.0, 1.0], thick(3.0))?;
        ax.plot_with([0.0, 1.0], [1.0, 0.0], thick(1.0))?;
        ax.plot_with([0.0, 1.0], [0.5, 0.5], thick(2.0))?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/clip_on_false_markers", |fig| {
        let ax = fig.subplot();
        let style = LineProps::fmt("o-")?.markersize(20.0).clip_on(false);
        ax.plot_with([0.0, 1.0, 2.0], [0.0, 1.0, 0.0], style)?;
        ax.set_xlim(0.0, 2.0)?;
        ax.set_ylim(0.0, 1.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("lines/nan_and_inf_gaps", |fig| {
        let ax = fig.subplot();
        let t = arange(0.0, 20.0, 1.0);
        let mut y = map(&t, |v| (v / 3.0).sin());
        for i in [4, 10, 11, 17] {
            y[i] = f64::NAN;
        }
        y[7] = f64::INFINITY;
        ax.plot_fmt(&t, &y, "o-")?;
        let steps = props().drawstyle(DrawStyle::StepsMid);
        ax.plot_with(&t, map(&y, |v| v + 2.0), steps)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/long_noisy_line_simplified", |fig| {
        let t = linspace(0.0, 10.0, 50_000);
        let noise = SpecRng::new(1).normal(t.len(), 0.0, 0.1);
        let y: Vec<f64> = t.iter().zip(noise).map(|(v, n)| v.sin() + n).collect();
        fig.subplot().plot_with(&t, y, props().linewidth(0.5))?;
        Ok(())
    }),
    FigureCase::new("lines/loglog", |fig| {
        let ax = fig.subplot();
        let t = logspace(-2.0, 3.0, 60, 10.0);
        ax.loglog(&t, map(&t, |v| v * v))?
            .set_linestyle(LineStyle::Solid)
            .set_marker("o".parse::<Marker>()?)
            .set_markersize(3.0);
        ax.loglog(&t, map(&t, |v| 10.0 * v.sqrt()))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/semilogy_exponential", |fig| {
        let ax = fig.subplot();
        let t = linspace(0.0, 10.0, 40);
        ax.semilogy(&t, map(&t, f64::exp))?;
        ax.semilogy(&t, map(&t, |v| (v / 2.0).exp()))?
            .set_linestyle(LineStyle::Dashed);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/semilogx_base2", |fig| {
        let ax = fig.subplot();
        let t: Vec<f64> = (0..12).map(|i| 2f64.powi(i)).collect();
        ax.set_scale(AxisName::X, Box::new(LogScale::new(2.0)?));
        ax.plot_fmt(&t, map(&t, f64::log2), "s-")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/single_point_and_constant", |fig| {
        let axes = fig.subplots(1, 2)?;
        let axes = axes.as_slice();
        axes[0].plot_fmt([3.0], [5.0], "o")?;
        axes[1].plot([0.0, 1.0, 2.0, 3.0], [2.0; 4])?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("lines/step_where", |fig| {
        let ax = fig.subplot();
        let t = arange(0.0, 10.0, 1.0);
        let y = map(&t, |v| (v / 2.0).sin());
        ax.step(&t, map(&y, |v| v + 4.0), StepWhere::Pre, props())?;
        ax.step(&t, map(&y, |v| v + 2.0), StepWhere::Mid, props())?;
        ax.step(&t, &y, StepWhere::Post, props())?;
        ax.plot_with(&t, &y, LineProps::fmt("o")?.color("0.6"))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/stairs_default_and_fill", |fig| {
        let ax = fig.subplot();
        let values = [1.0, 3.0, 2.0, 5.0, 4.0, 2.0];
        ax.stairs(values, StairsOptions::default())?
            .set_linewidth(2.0);
        let filled = StairsOptions {
            edges: Some(vec![0.0, 1.0, 1.5, 3.0, 4.0, 5.5, 6.0]),
            fill: true,
            ..StairsOptions::default()
        };
        ax.stairs(map(&values, |v| v + 6.0), filled)?
            .set_alpha(Some(0.5));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/stairs_baseline_and_horizontal", |fig| {
        let axes = fig.subplots(1, 2)?;
        let axes = axes.as_slice();
        let values = [3.0, 1.0, 4.0, 1.0, 5.0];
        let raised = StairsOptions {
            baseline: StepBaseline::Value(2.0),
            fill: true,
            ..StairsOptions::default()
        };
        axes[0].stairs(values, raised)?.set_hatch("//");
        let open = StairsOptions {
            baseline: StepBaseline::None,
            color: Some("k".into()),
            ..StairsOptions::default()
        };
        axes[0].stairs(values, open)?;
        let sideways = StairsOptions {
            orientation: Orientation::Horizontal,
            fill: true,
            ..StairsOptions::default()
        };
        axes[1].stairs(values, sideways)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("lines/vlines_hlines", |fig| {
        let ax = fig.subplot();
        let t = arange(1.0, 8.0, 1.0);
        ax.vlines(&t, [0.0], map(&t, f64::sqrt))?
            .set_edgecolor("C0")
            .set_linewidths(vec![3.0]);
        ax.hlines([0.5, 1.5, 2.5], [1.0], [3.0, 5.0, 7.0])?
            .set_edgecolors(pltrs::collections::EdgeColors::Colors(vec![
                "r".into(),
                "g".into(),
                "b".into(),
            ]))
            .set_linestyles(vec![LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted]);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/axhline_axvline_fractions", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 10.0], [0.0, 5.0])?;
        ax.axhline(2.5, props().color("k"));
        let dashed = props()
            .color("r")
            .linestyle(LineStyle::Dashed)
            .linewidth(2.0);
        ax.axhline_between(4.0, 0.25, 0.75, dashed);
        ax.axvline(3.0, props().color("g").linewidth(3.0));
        ax.axvline_between(7.0, 0.1, 0.4, props().color("m"));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/axline_points_and_slope", |fig| {
        let ax = fig.subplot();
        ax.plot_fmt([0.0, 4.0], [0.0, 3.0], "o")?;
        let origin = Point::new(0.0, 0.0);
        ax.axline(
            origin,
            AxLineThrough::Point(Point::new(1.0, 1.0)),
            props().color("C1"),
        )?;
        let dashed = props().color("C2").linestyle(LineStyle::Dashed);
        ax.axline(Point::new(0.0, 1.0), AxLineThrough::Slope(-0.5), dashed)?;
        let vertical = AxLineThrough::Point(Point::new(2.0, 1.0));
        ax.axline(Point::new(2.0, 0.0), vertical, props().color("C3"))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("lines/axhspan_axvspan", |fig| {
        let ax = fig.subplot();
        let x = big_x();
        ax.plot(&x, map(&x, f64::sin))?;
        ax.axhspan(-0.25, 0.25)
            .set_color("yellow")
            .set_alpha(Some(0.5));
        ax.axvspan(1.0, 2.0)
            .set_color("tab:green")
            .set_alpha(Some(0.3));
        ax.axvspan_between(4.0, 5.0, 0.2, 0.6)
            .set_facecolor("none")
            .set_edgecolor("r")
            .set_hatch("x");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
