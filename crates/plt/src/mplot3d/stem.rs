//! Stem plots in 3-D (matplotlib's `Axes3D.stem`).

use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use crate::axes::solid_format;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::lines::{FormatString, Line2D, LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};
use crate::units::AxisValues;

/// Label of the parts of a stem plot kept out of legends.
const NO_LEGEND: &str = "_nolegend_";

/// Options of [`Axes3D::stem`]; matplotlib's format strings parse with
/// [`str::parse`].
#[derive(Clone, Debug, PartialEq)]
pub struct Stem3DOptions {
    /// Colour and line style of the stems (`"C0-"`).
    pub linefmt: FormatString,
    /// Format of the markers at the heads (`"C0o"`).
    pub markerfmt: FormatString,
    /// Format of the baseline (`"C3-"`).
    pub basefmt: FormatString,
    /// Position of the baseline along `orientation`.
    pub bottom: f64,
    /// The axis the stems run along.
    pub orientation: Axis3,
    /// Whether the stems are hidden where they reach beyond the view
    /// limits; the markers and the baseline are not, as in matplotlib
    /// (`axlim_clip`).
    pub axlim_clip: bool,
}

impl Default for Stem3DOptions {
    fn default() -> Self {
        Self {
            linefmt: solid_format("C0"),
            markerfmt: FormatString {
                linestyle: Some(LineStyle::None),
                marker: Some(Marker::Symbol(MarkerSymbol::Circle)),
                color: Some(Color::from("C0")),
            },
            basefmt: solid_format("C3"),
            bottom: 0.0,
            orientation: Axis3::Z,
            axlim_clip: false,
        }
    }
}

/// The artists of a 3-D stem plot (matplotlib's `StemContainer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stem3D {
    /// The markers at the heads.
    pub markerline: Line2D,
    /// The stems.
    pub stemlines: Collection,
    /// The line through the stem feet.
    pub baseline: Line2D,
}

impl Axes3D {
    /// Stems from a baseline at `bottom` along the orientation axis to the
    /// heads `(x, y, z)`, with markers at the heads (matplotlib's `stem`).
    /// The heads are numbers, dates, category names or user values,
    /// converted on their axes as by [`Axes3D::plot`]; `bottom` is a
    /// number of the orientation axis.
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length.
    pub fn stem(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        options: Stem3DOptions,
    ) -> Result<Stem3D, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        check_len("y", values[0].len(), values[1].len())?;
        check_len("z", values[0].len(), values[2].len())?;
        let had_data = self.has_data();
        let [x, y, z] = self.convert_xyz(values);
        let span = |v: &[f64]| {
            let low = v.iter().copied().fold(f64::INFINITY, f64::min);
            let high = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (low, high)
        };
        let limits = [span(&x), span(&y), span(&z)];
        let bottom = options.bottom;
        let orientation = options.orientation.index();
        // The baseline lies in the plane of the two other axes.
        let (a, b) = match options.orientation {
            Axis3::X => (1, 2),
            Axis3::Y => (0, 2),
            Axis3::Z => (0, 1),
        };
        let columns = [&x, &y, &z];
        let stems: Vec<Vec<Vec3>> = (0..x.len())
            .map(|i| {
                let head = [x[i], y[i], z[i]];
                let mut foot = head;
                foot[orientation] = bottom;
                vec![foot, head]
            })
            .collect();
        let no_legend = |format: FormatString| LineProps::from(format).label(NO_LEGEND);
        let baseline = self.plot_with(
            columns[a],
            columns[b],
            [bottom],
            options.orientation,
            no_legend(options.basefmt),
        )?;
        let format = options.linefmt;
        let mut props = CollectionProps::default().label(NO_LEGEND);
        props.color = format.color;
        props.linestyles =
            Some(vec![format.linestyle.unwrap_or_else(|| {
                crate::rc::params().lines_linestyle.clone()
            })]);
        let stemlines = self.add_segments(stems, props, options.axlim_clip);
        let markerline = self.plot_with(&x, &y, &z, Axis3::Z, no_legend(options.markerfmt))?;
        let base_span = |axis: usize| -> [f64; 2] {
            let (low, high) = limits[axis];
            [low, high]
        };
        let mut feet = [[0.0; 2]; 3];
        feet[a] = base_span(a);
        feet[b] = base_span(b);
        feet[orientation] = [bottom, bottom];
        let extent = |axis: usize| {
            let (low, high) = limits[axis];
            vec![feet[axis][0], feet[axis][1], low, high]
        };
        self.auto_scale_xyz(&extent(0), &extent(1), Some(&extent(2)), had_data)?;
        Ok(Stem3D {
            markerline,
            stemlines,
            baseline,
        })
    }
}
