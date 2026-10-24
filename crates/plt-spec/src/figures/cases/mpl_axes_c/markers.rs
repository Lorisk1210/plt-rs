//! Marker tests: stems, transparent and RGBA markers, alpha on lines and
//! markers, every marker style, fill styles from the rcParams and vertex
//! markers; and the Mollweide grid.

use plt::axes::StemOptions;
use plt::geo::GeoProjection;
use plt::legend::LegendOptions;
use plt::lines::{FormatError, LineProps, LineStyle};
use plt::markers::{Marker, MarkerError};
use plt::numeric::{arange, linspace};
use plt::orientation::Orientation;
use plt::transforms::Point;

use super::{classic, map, remove_ticks_and_titles};
use crate::figures::FigureCase;
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_c/stem", |fig| {
        let x = linspace(0.1, 2.0 * std::f64::consts::PI, 100);
        let ax = fig.subplot();
        let options = StemOptions {
            label: Some("stem".into()),
            ..dash_dotted_stems("k+")?
        };
        ax.stem(&x, map(&x, f64::cos), options)?;
        ax.legend(LegendOptions::default())?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/stem_orientation", |fig| {
        let x = linspace(0.1, 2.0 * std::f64::consts::PI, 50);
        let options = StemOptions {
            orientation: Orientation::Horizontal,
            ..dash_dotted_stems("kx")?
        };
        fig.subplot().stem(&x, map(&x, f64::cos), options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/transparent_markers", |fig| {
        let data = SpecRng::new(0).uniform(50, 0.0, 1.0);
        let props = LineProps::fmt("D")?
            .markerfacecolor("none")
            .markersize(100.0);
        fig.subplot().plot_with(indices(data.len()), data, props)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/rgba_markers", |fig| {
        let axs = fig.subplots(1, 2)?;
        let red = [(1.0, 0.0, 0.0, 1.0), (1.0, 0.0, 0.0, 0.5)];
        let blue = [(0.0, 0.0, 1.0, 1.0), (0.0, 0.0, 1.0, 0.5)];
        for (ax, alpha) in axs.as_slice().iter().zip([None, Some(0.2)]) {
            for (j, &rcolor) in red.iter().enumerate() {
                for (k, &bcolor) in blue.iter().enumerate() {
                    let (x, y) = (j as f64 + 1.0, k as f64 + 1.0);
                    let big = |format: &str| -> Result<LineProps, FormatError> {
                        let props = LineProps::fmt(format)?
                            .markeredgecolor(rcolor)
                            .markersize(100.0)
                            .markeredgewidth(20.0);
                        Ok(match alpha {
                            Some(alpha) => props.alpha(alpha),
                            None => props,
                        })
                    };
                    ax.plot_with([x], [y], big("o")?.markerfacecolor(bcolor))?;
                    ax.plot_with([x], [y + 2.0], big("x")?)?;
                }
            }
        }
        for ax in axs.as_slice() {
            ax.set_xlim(-1.0, 4.0)?;
            ax.set_ylim(0.0, 5.0)?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/alpha", |fig| {
        let data = SpecRng::new(0).uniform(50, 0.0, 1.0);
        let ax = fig.subplot();
        let red = (1.0, 0.0, 0.0, 1.0);
        let translucent = (1.0, 0.0, 0.0, 0.5);
        let lines = [
            (0.0, red, translucent, None),
            (2.0, translucent, translucent, Some(1.0)),
            (4.0, red, red, Some(0.5)),
            (6.0, translucent, translucent, None),
            (8.0, translucent, red, None),
        ];
        for (offset, color, face, alpha) in lines {
            let props = LineProps::fmt("-D")?
                .color(color)
                .markerfacecolor(face)
                .markersize(20.0)
                .linewidth(10.0);
            let props = match alpha {
                Some(alpha) => props.alpha(alpha),
                None => props,
            };
            ax.plot_with(indices(data.len()), map(&data, |y| y + offset), props)?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/marker_styles", |fig| {
        let ax = fig.subplot();
        for (y, marker) in (0_u32..).zip(every_marker()?) {
            let y = f64::from(y);
            let x = map(&arange(0.0, 10.0, 1.0), |i| (y % 2.0) * 5.0 + i * 10.0);
            let props = LineProps::default()
                .linestyle(LineStyle::None)
                .marker(marker)
                .markersize(10.0 + y / 5.0);
            ax.plot_with(x, vec![10.0 * y; 10], props)?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/markers_fillstyle_rcparams", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 7.0, 1.0);
        for (idx, (style, marker)) in [("top", "s"), ("bottom", "o"), ("none", "^")]
            .into_iter()
            .enumerate()
        {
            plt::rc::set("markers.fillstyle", style)?;
            let shift = idx as f64;
            let props = LineProps::default().marker(marker.parse::<Marker>()?);
            ax.plot_with(x.clone(), map(&x, |x| x + shift), props)?;
        }
        Ok(())
    }),
    classic("mpl_axes_c/vertex_markers", |fig| {
        let data = arange(0.0, 10.0, 1.0);
        let square = Marker::Vertices(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]);
        let ax = fig.subplot();
        for (y, face) in [
            (data.clone(), "k"),
            (data.iter().rev().copied().collect(), "b"),
        ] {
            let props = LineProps::default()
                .linestyle(LineStyle::None)
                .marker(square.clone())
                .markerfacecolor(face);
            ax.plot_with(data.clone(), y, props)?;
        }
        ax.set_xlim(-1.0, 10.0)?;
        ax.set_ylim(-1.0, 10.0)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/mollweide_grid", |fig| {
        fig.geo_subplot(GeoProjection::Mollweide).grid(true);
        remove_ticks_and_titles(fig);
        Ok(())
    }),
];

/// `linefmt="C2-.", basefmt="C1-."` with the given marker format.
fn dash_dotted_stems(markerfmt: &str) -> Result<StemOptions, FormatError> {
    Ok(StemOptions {
        linefmt: "C2-.".parse()?,
        markerfmt: markerfmt.parse()?,
        basefmt: Some("C1-.".parse()?),
        ..StemOptions::default()
    })
}

/// `0, 1, ..., n - 1`, the x values of `plot(y)`.
fn indices(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64).collect()
}

/// `MarkerStyle.markers` in the order of `test_marker_styles`: sorted by
/// `str(type(m)) + str(m)` (integer codes first, as text), with `"none"`
/// moved to the front.
fn every_marker() -> Result<Vec<Marker>, MarkerError> {
    const CODES: [i64; 12] = [0, 1, 10, 11, 2, 3, 4, 5, 6, 7, 8, 9];
    const NAMES: [&str; 28] = [
        "", " ", "*", "+", ",", ".", "1", "2", "3", "4", "8", "<", ">", "D", "H", "None", "P", "X",
        "^", "_", "d", "h", "o", "p", "s", "v", "x", "|",
    ];
    std::iter::once("none".parse())
        .chain(CODES.into_iter().map(Marker::from_code))
        .chain(NAMES.into_iter().map(str::parse))
        .collect()
}
