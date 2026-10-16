//! Bar charts (matplotlib's `bar` and `barh`).

use super::broadcast::{broadcast, common_length};
use super::children::DataUpdate;
use super::{Axes, AxesState, ErrorbarContainer, ErrorbarOptions, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::orientation::Orientation;
use crate::patches::{Patch, PatchArtist, PatchProps, Rectangle};
use crate::scale::{LogScale, NonPositive};
use crate::ticker::AxisName;
use crate::units::AxisValues;

/// Where a bar sits relative to its position (matplotlib's `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BarAlign {
    /// Centred on the position.
    #[default]
    Center,
    /// Starting at the position; a negative thickness extends backwards.
    Edge,
}

/// Options of [`Axes::bar`] and [`Axes::barh`] (matplotlib's keywords).
///
/// Per-bar lists hold one value for all bars or one value per bar;
/// colour lists are cycled over the bars.
#[derive(Clone, Debug)]
pub struct BarOptions {
    /// Thickness across the value direction: the width of vertical bars,
    /// the height of horizontal ones (default 0.8).
    pub thickness: Vec<f64>,
    /// Where the bars start along the value direction: `bottom` of `bar`,
    /// `left` of `barh` (default 0).
    pub base: Vec<f64>,
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
            thickness: vec![0.8],
            base: vec![0.0],
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
        height: impl AsRef<[f64]>,
        options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        self.bars(Orientation::Vertical, x.into(), height.as_ref(), options)
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
        width: impl AsRef<[f64]>,
        options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        self.bars(Orientation::Horizontal, y.into(), width.as_ref(), options)
    }

    fn bars(
        &self,
        orientation: Orientation,
        positions: AxisValues,
        values: &[f64],
        options: BarOptions,
    ) -> Result<BarContainer, PlotError> {
        let (position_axis, value_axis) = match orientation {
            Orientation::Vertical => (AxisName::X, AxisName::Y),
            Orientation::Horizontal => (AxisName::Y, AxisName::X),
        };
        if options.log {
            let log = LogScale::new(10.0)
                .expect("10 is a valid logarithm base")
                .with_nonpositive(NonPositive::Clip);
            self.set_scale(value_axis, Box::new(log));
        }
        let positions = self.axis_of(position_axis).convert_units(positions);
        let n = common_length(&[
            ("x", positions.len()),
            ("height", values.len()),
            ("width", options.thickness.len()),
            ("y", options.base.len()),
            ("linewidth", options.linewidth.len().max(1)),
            ("hatch", options.hatch.len().max(1)),
        ])?;
        let positions = broadcast("x", &positions, n)?;
        let values = broadcast("height", values, n)?;
        let thickness = broadcast("width", &options.thickness, n)?;
        let base = broadcast("y", &options.base, n)?;
        if !options.bar_labels.is_empty() && options.bar_labels.len() != n {
            return Err(PlotError::LabelCount {
                labels: options.bar_labels.len(),
                items: n,
            });
        }
        let geometry: Vec<BarGeometry> = (0..n)
            .map(|i| BarGeometry {
                start: match options.align {
                    BarAlign::Center => positions[i] - thickness[i] / 2.0,
                    BarAlign::Edge => positions[i],
                },
                thickness: thickness[i],
                base: base[i],
                value: values[i],
            })
            .collect();
        let faces = if !options.color.is_empty() {
            options.color.clone()
        } else if let Some(face) = &options.props.facecolor {
            vec![face.clone()]
        } else {
            vec![self.next_patch_color()]
        };
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
            let axis = self.axis_of(position_axis);
            axis.set_ticks(positions.clone())
                .set_ticklabels(tick_labels)?;
        }
        let label = if options.bar_labels.is_empty() {
            options.label
        } else {
            Some("_nolegend_".to_owned())
        };
        Ok(BarContainer {
            patches,
            errorbar,
            datavalues: values,
            orientation,
            label,
        })
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
        patch.base_mut().transform = Some(self.trans_data());
        let update = DataUpdate {
            points: patch.data_points(),
            update: (true, true),
            autoscale: (false, false),
        };
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
        "none".clone_into(&mut errors.fmt);
        errors.ecolor.get_or_insert_with(|| Color::from("k"));
        let base = &mut errors.line.base;
        if base.zorder.is_none() {
            base.zorder = props.base.zorder.map(|z| z + 0.01);
        }
        base.label.get_or_insert_with(|| "_nolegend_".to_owned());
        self.errorbar(x, y, errors)
    }
}
