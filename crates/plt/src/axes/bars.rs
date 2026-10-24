//! Bar charts (matplotlib's `bar` and `barh`).

use plt_core::{Path, Point};

use super::broadcast::{broadcast, common_length};
use super::children::DataUpdate;
use super::{
    Axes, AxesState, ErrorSize, ErrorbarContainer, ErrorbarFormat, ErrorbarOptions, PlotError,
};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind};
use crate::colors::Color;
use crate::orientation::Orientation;
use crate::patches::{Patch, PatchArtist, PatchProps, Rectangle};
use crate::scale::{LogScale, NonPositive};
use crate::ticker::AxisName;
use crate::units::{AxisDelta, AxisDeltas, AxisValue, AxisValues};

/// Where a bar sits relative to its position (matplotlib's `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BarAlign {
    /// Centred on the position.
    #[default]
    Center,
    /// Starting at the position; a negative thickness extends backwards.
    Edge,
}

/// Where a [`Axes::broken_barh`] band sits relative to its position
/// (matplotlib's `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BandAlign {
    /// The band starts at the position.
    #[default]
    Bottom,
    /// The band is centred on the position.
    Center,
    /// The band ends at the position.
    Top,
}

/// The vertical extent of an [`Axes::broken_barh`] band (matplotlib's
/// `yrange`): a position on the y axis (a number, a category name, a date
/// or a user value) and a height, an extent from it.
///
/// ```
/// use plt::axes::BandRange;
/// assert_ne!(BandRange::from((20.0, 9.0)), BandRange::from(("CPU 1", 9.0)));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BandRange {
    /// Where the band sits.
    position: AxisValue,
    /// The extent of the band along y.
    height: AxisDelta,
}

impl<P: Into<AxisValue>, H: Into<AxisDelta>> From<(P, H)> for BandRange {
    fn from((position, height): (P, H)) -> Self {
        Self {
            position: position.into(),
            height: height.into(),
        }
    }
}

/// Options of [`Axes::bar`] and [`Axes::barh`] (matplotlib's keywords).
///
/// Per-bar lists hold one value for all bars or one value per bar;
/// colour lists are cycled over the bars.
#[derive(Clone, Debug)]
pub struct BarOptions {
    /// Thickness across the value direction: the width of vertical bars,
    /// the height of horizontal ones (default 0.8), an extent from the bar
    /// positions.
    pub thickness: AxisDeltas,
    /// Where the bars start along the value direction: `bottom` of `bar`,
    /// `left` of `barh` (default 0).
    pub base: AxisValues,
    /// Placement relative to the positions.
    pub align: BarAlign,
    /// Face colours (default: the next colour of the property cycle).
    pub color: Vec<Color>,
    /// Edge colours (default: the patch default).
    pub edgecolor: Vec<Color>,
    /// Edge widths in points.
    pub linewidth: Vec<f64>,
    /// Hatch patterns.
    pub hatch: Vec<String>,
    /// Properties applied to every bar after the per-bar ones.
    pub props: PatchProps,
    /// Legend label of the whole bar container.
    pub label: Option<String>,
    /// One legend label per bar, replacing the container's `label`.
    pub bar_labels: Vec<String>,
    /// Tick labels at the bar positions.
    pub tick_labels: Vec<String>,
    /// Use a log scale (non-positive values clipped) on the value axis.
    pub log: bool,
    /// Error bars: drawn when `xerr` or `yerr` is set; `ecolor` defaults
    /// to black and `capsize` to `errorbar.capsize`.
    pub errors: ErrorbarOptions,
}

impl Default for BarOptions {
    fn default() -> Self {
        Self {
            thickness: AxisDeltas::Numbers(vec![0.8]),
            base: AxisValues::Numbers(vec![0.0]),
            align: BarAlign::Center,
            color: Vec::new(),
            edgecolor: Vec::new(),
            linewidth: Vec::new(),
            hatch: Vec::new(),
            props: PatchProps::default(),
            label: None,
            bar_labels: Vec::new(),
            tick_labels: Vec::new(),
            log: false,
            errors: ErrorbarOptions::default(),
        }
    }
}

/// The artists of a bar chart (matplotlib's `BarContainer`).
#[derive(Clone, Debug)]
pub struct BarContainer {
    /// One rectangle per bar.
    pub patches: Vec<Patch<Rectangle>>,
    /// The error bars, if any were requested.
    pub errorbar: Option<ErrorbarContainer>,
    /// The bar values (heights of vertical bars, widths of horizontal ones).
    pub datavalues: Vec<f64>,
    /// Which way the bars grow.
    pub orientation: Orientation,
    /// Legend label.
    pub label: Option<String>,
}

impl BarContainer {
    /// Each bar's start and extent along its position and its value
    /// direction: `((x, width), (y, height))` of vertical bars swapped for
    /// horizontal ones.
    fn spans(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + '_ {
        self.patches.iter().map(|patch| {
            let Rectangle {
                xy, width, height, ..
            } = patch.shape();
            match self.orientation {
                Orientation::Vertical => ((xy.x, width), (xy.y, height)),
                Orientation::Horizontal => ((xy.y, height), (xy.x, width)),
            }
        })
    }

    /// The value at the base of each bar (matplotlib's `bottoms`).
    pub fn bottoms(&self) -> Vec<f64> {
        self.spans().map(|(_, (base, _))| base).collect()
    }

    /// The value at the end of each bar (matplotlib's `tops`).
    pub fn tops(&self) -> Vec<f64> {
        self.spans()
            .map(|(_, (base, value))| base + value)
            .collect()
    }

    /// The position of the middle of each bar across the value direction
    /// (matplotlib's `position_centers`).
    pub fn position_centers(&self) -> Vec<f64> {
        self.spans()
            .map(|((start, thickness), _)| start + thickness / 2.0)
            .collect()
    }
}

/// The per-bar arguments in axis numbers, one or one per bar.
struct BarNumbers {
    /// The positions.
    starts: Vec<f64>,
    /// Where the bars start along the value direction.
    bases: Vec<f64>,
    /// The thicknesses across the value direction.
    thickness: Vec<f64>,
    /// The values (heights of vertical bars, widths of horizontal ones).
    values: Vec<f64>,
}

/// One bar's geometry along the position and value directions.
struct BarGeometry {
    start: f64,
    thickness: f64,
    base: f64,
    value: f64,
}

impl BarGeometry {
    fn rectangle(&self, orientation: Orientation) -> Rectangle {
        let (x, y) = orientation.place(self.start, self.base);
        let (width, height) = orientation.place(self.thickness, self.value);
        Rectangle::new((x, y), width, height)
    }

    /// Where the error bar attaches: the middle of the far edge.
    fn tip(&self) -> (f64, f64) {
        (self.start + 0.5 * self.thickness, self.base + self.value)
    }
}

impl Axes {
    /// Vertical bars at `x` with heights `height` (matplotlib's `bar`).
    ///
    /// The heights, the widths (`options.thickness`) and the error sizes
    /// are extents: numbers, durations or user values, converted through
    /// the values they extend from, the bottoms (`options.base`) for
    /// heights and the positions for widths, as matplotlib's `_convert_dx`
    /// converts them. When the bottoms give the value axis no unit, the
    /// heights may.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::BarOptions;
    /// let ax = Figure::new().subplot();
    /// let bars = ax.bar([1.0, 2.0], [3.0, 4.0], BarOptions::default())?;
    /// assert_eq!(bars.patches.len(), 2);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When per-bar arguments do not broadcast or the error bars are
    /// invalid.
    pub fn bar(
        &self,
        x: impl Into<AxisValues>,
        height: impl Into<AxisDeltas>,
        options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        self.bars(Orientation::Vertical, x.into(), &height.into(), options)
    }

    /// Horizontal bars at `y` with widths `width` (matplotlib's `barh`);
    /// `options.thickness` is the bar height and `options.base` the left
    /// edge.
    ///
    /// # Errors
    ///
    /// As [`Axes::bar`].
    pub fn barh(
        &self,
        y: impl Into<AxisValues>,
        width: impl Into<AxisDeltas>,
        options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        self.bars(Orientation::Horizontal, y.into(), &width.into(), options)
    }

    /// A horizontal band of rectangles, one per `(start, width)` in
    /// `xranges`, spanning the band's height vertically at its position on
    /// the y axis (matplotlib's `broken_barh`). The starts may be dates
    /// or user values, with durations or user values for widths: each
    /// rectangle ends where its width takes its start, or, where the
    /// width does not add to the start (which matplotlib refuses), at the
    /// start plus the width converted by itself. The band height converts
    /// through the band position (matplotlib's `_convert_dx`). Style the
    /// band through the returned collection.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::BandAlign;
    /// let ax = Figure::new().subplot();
    /// let band = ax.broken_barh(&[(10.0, 50.0), (100.0, 20.0)], (20.0, 9.0), BandAlign::Bottom);
    /// band.set_facecolor("tab:blue");
    ///
    /// let ax = Figure::new().subplot();
    /// ax.broken_barh(&[(0.0, 3.0)], ("CPU 1", 0.4), BandAlign::Center);
    /// ax.broken_barh(&[(1.0, 1.5)], ("disk", 0.4), BandAlign::Center);
    /// assert_eq!(ax.yaxis().ticklocs(false)?, [0.0, 1.0]);
    /// # Ok::<(), plt::ticker::TickerError>(())
    /// ```
    pub fn broken_barh<X, W>(
        &self,
        xranges: &[(X, W)],
        band: impl Into<BandRange>,
        align: BandAlign,
    ) -> Collection
    where
        X: Clone + Into<AxisValue>,
        W: Clone + Into<AxisDelta>,
    {
        let BandRange { position, height } = band.into();
        let yaxis = self.yaxis();
        let origin = AxisValues::from(position);
        let converted = yaxis.convert_units(origin.clone());
        let height = yaxis.convert_dx(&height.into(), &origin, &converted)[0];
        let position = converted[0];
        let (y0, y1) = match align {
            BandAlign::Bottom => (position, position + height),
            BandAlign::Center => (position - height / 2.0, position + height / 2.0),
            BandAlign::Top => (position - height, position),
        };
        let xaxis = self.xaxis();
        let paths = xranges
            .iter()
            .map(|(x0, width)| {
                let (x0, x1) = xaxis.convert_span(x0.clone().into(), &width.clone().into());
                Path::polygon([
                    Point::new(x0, y0),
                    Point::new(x0, y1),
                    Point::new(x1, y1),
                    Point::new(x1, y0),
                ])
            })
            .collect();
        let mut band = CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
        band.base_mut().transform = Some(self.trans_data());
        let update = DataUpdate::limits(band.shape_extents());
        Collection::from_raw(self.adopt_collection(band, &update))
    }

    fn bars(
        &self,
        orientation: Orientation,
        positions: AxisValues,
        values: &AxisDeltas,
        mut options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        let (position_name, value_name) = bar_axes(orientation);
        let faces = if !options.color.is_empty() {
            options.color.clone()
        } else if let Some(face) = &options.props.facecolor {
            vec![face.clone()]
        } else {
            vec![self.next_patch_color()]
        };
        if options.log {
            let log = LogScale::new(10.0)
                .expect("10 is a valid logarithm base")
                .with_nonpositive(NonPositive::Clip);
            self.set_scale(value_name, Box::new(log));
        }
        let numbers = self.bar_numbers(orientation, &positions, values, &mut options);
        let geometry = layout_bars(&numbers, &options)?;
        let n = geometry.len();
        let patches = geometry
            .iter()
            .enumerate()
            .map(|(i, bar)| self.add_bar(orientation, bar, i, &faces, &options))
            .collect();
        let errorbar = if options.errors.xerr.is_some() || options.errors.yerr.is_some() {
            Some(self.bar_errors(orientation, &geometry, options.errors, &options.props)?)
        } else {
            None
        };
        self.raw()
            .with_mut::<AxesState, _>(|axes| axes.request_autoscale(true, true));
        if !options.tick_labels.is_empty() {
            let tick_labels = broadcast("tick_label", &options.tick_labels, n)?;
            let ticks = broadcast("x", &numbers.starts, n)?;
            self.axis_of(position_name)
                .set_ticks(ticks)
                .set_ticklabels(tick_labels)?;
        }
        let label = if options.bar_labels.is_empty() {
            options.label
        } else {
            Some("_nolegend_".to_owned())
        };
        let container = BarContainer {
            patches,
            errorbar,
            datavalues: geometry.iter().map(|bar| bar.value).collect(),
            orientation,
            label,
        };
        self.add_container(container.clone());
        Ok(container)
    }

    /// The bar arguments in axis numbers, with the error sizes of `options`
    /// converted in place (matplotlib's `_process_unit_info` and
    /// `_convert_dx` in `bar`). The positions and the bases convert on
    /// their axes, the values giving the value axis its unit when the
    /// bases did not; the extents convert through what they extend from:
    /// the thicknesses and the errors across the bars through the
    /// positions, the values and the errors along the bars through the
    /// bases.
    fn bar_numbers(
        &self,
        orientation: Orientation,
        positions: &AxisValues,
        values: &AxisDeltas,
        options: &mut BarOptions,
    ) -> BarNumbers {
        let (position_name, value_name) = bar_axes(orientation);
        let (position_axis, value_axis) = (self.axis_of(position_name), self.axis_of(value_name));
        let starts = position_axis.convert_units(positions.clone());
        let bases = value_axis.convert_units(options.base.clone());
        if let AxisDeltas::Custom(values) = values
            && !value_axis.have_units()
        {
            value_axis.update_units(values.clone());
        }
        let across = |err: &ErrorSize| {
            err.converted(|deltas| position_axis.convert_dx(deltas, positions, &starts))
        };
        let along = |err: &ErrorSize| {
            err.converted(|deltas| value_axis.convert_dx(deltas, &options.base, &bases))
        };
        let errors = &options.errors;
        let (xerr, yerr) = match orientation {
            Orientation::Vertical => (
                errors.xerr.as_ref().map(across),
                errors.yerr.as_ref().map(along),
            ),
            Orientation::Horizontal => (
                errors.xerr.as_ref().map(along),
                errors.yerr.as_ref().map(across),
            ),
        };
        let numbers = BarNumbers {
            thickness: position_axis.convert_dx(&options.thickness, positions, &starts),
            values: value_axis.convert_dx(values, &options.base, &bases),
            starts,
            bases,
        };
        (options.errors.xerr, options.errors.yerr) = (xerr, yerr);
        numbers
    }

    /// Adds bar `index`: per-bar styles first, then the shared properties;
    /// the bar's base is sticky so margins do not extend past it.
    fn add_bar(
        &self,
        orientation: Orientation,
        bar: &BarGeometry,
        index: usize,
        faces: &[Color],
        options: &BarOptions,
    ) -> Patch<Rectangle> {
        let mut patch = PatchArtist::new(bar.rectangle(orientation), crate::rc::params());
        // Bars follow the circle on polar axes.
        patch.interpolation_steps = 100;
        patch.style.facecolor = Some(faces[index % faces.len()].clone());
        if !options.edgecolor.is_empty() {
            patch.style.edgecolor =
                Some(options.edgecolor[index % options.edgecolor.len()].clone());
        }
        if !options.linewidth.is_empty() {
            patch.style.linewidth = Some(options.linewidth[index % options.linewidth.len()]);
        }
        if !options.hatch.is_empty() {
            patch.style.hatch = Some(options.hatch[index % options.hatch.len()].clone());
        }
        patch.base_mut().label = options
            .bar_labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| "_nolegend_".to_owned());
        options.props.clone().apply(&mut patch);
        let sticky = &mut patch.base_mut().sticky_edges;
        match orientation {
            Orientation::Vertical => sticky.1.push(bar.base),
            Orientation::Horizontal => sticky.0.push(bar.base),
        }
        patch
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        let update = DataUpdate::extending(patch.limit_points(self.id()));
        Patch::from_raw(self.adopt(patch, &update))
    }

    /// Error bars at the bar tips (matplotlib's `error_kw` handling).
    fn bar_errors(
        &self,
        orientation: Orientation,
        geometry: &[BarGeometry],
        mut errors: ErrorbarOptions,
        props: &PatchProps,
    ) -> Result<ErrorbarContainer, PlotError> {
        let (x, y): (Vec<f64>, Vec<f64>) = geometry
            .iter()
            .map(|bar| {
                let (position, value) = bar.tip();
                orientation.place(position, value)
            })
            .unzip();
        errors.fmt = ErrorbarFormat::None;
        errors.ecolor.get_or_insert_with(|| Color::from("k"));
        let base = &mut errors.line.base;
        if base.zorder.is_none() {
            base.zorder = props.base.zorder.map(|z| z + 0.01);
        }
        base.label.get_or_insert_with(|| "_nolegend_".to_owned());
        self.errorbar(x, y, errors)
    }
}

/// The axes bars of `orientation` are placed along and grow along.
fn bar_axes(orientation: Orientation) -> (AxisName, AxisName) {
    match orientation {
        Orientation::Vertical => (AxisName::X, AxisName::Y),
        Orientation::Horizontal => (AxisName::Y, AxisName::X),
    }
}

/// Broadcasts the per-bar arguments to a common length and places the
/// bars (matplotlib's `np.broadcast_arrays` and alignment in `bar`).
fn layout_bars(numbers: &BarNumbers, options: &BarOptions) -> Result<Vec<BarGeometry>, PlotError> {
    let BarNumbers {
        starts: positions,
        bases,
        thickness,
        values,
    } = numbers;
    let n = common_length(&[
        ("x", positions.len()),
        ("height", values.len()),
        ("width", thickness.len()),
        ("y", bases.len()),
        ("linewidth", options.linewidth.len().max(1)),
        ("hatch", options.hatch.len().max(1)),
    ])?;
    if !options.bar_labels.is_empty() && options.bar_labels.len() != n {
        return Err(PlotError::LabelCount {
            labels: options.bar_labels.len(),
            items: n,
        });
    }
    let positions = broadcast("x", positions, n)?;
    let values = broadcast("height", values, n)?;
    let thickness = broadcast("width", thickness, n)?;
    let base = broadcast("y", bases, n)?;
    Ok((0..n)
        .map(|i| BarGeometry {
            start: match options.align {
                BarAlign::Center => positions[i] - thickness[i] / 2.0,
                BarAlign::Edge => positions[i],
            },
            thickness: thickness[i],
            base: base[i],
            value: values[i],
        })
        .collect())
}
