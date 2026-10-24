//! Sankey diagrams: flows drawn as arrows whose widths are proportional to
//! their quantities (matplotlib.sankey).
//!
//! A [`Sankey`] collects subdiagrams on one axes. Each
//! [`Sankey::add`] call draws a trunk with its inputs (positive flows,
//! entering through dips) and outputs (negative flows, leaving through
//! arrow heads) on the left/right, top or bottom, and may connect one of
//! its flows to a flow of an earlier subdiagram. [`Sankey::finish`] fits
//! the view around everything with equal scaling.
//!
//! ```
//! use plt::prelude::*;
//! use plt::sankey::{Sankey, SankeyOptions, Subdiagram};
//! let ax = Figure::new().subplot();
//! let mut sankey = Sankey::new(&ax, SankeyOptions::default())?;
//! let diagrams = sankey
//!     .add(Subdiagram { flows: vec![0.6, 0.4, -1.0], ..Subdiagram::default() })?
//!     .finish()?;
//! assert_eq!(diagrams[0].tips.len(), 3);
//! # Ok::<(), plt::sankey::SankeyError>(())
//! ```

mod error;
mod options;
mod outline;

use plt_core::{Affine2D, Path, Point};

pub use error::SankeyError;
pub use options::{FlowOrientation, PathLengths, SankeyDiagram, SankeyOptions, Subdiagram};
use outline::{Direction, Geometry, PlacedFlow};

use crate::axes::{Adjustable, Axes};
use crate::colors::Color;
use crate::patches::{PatchProps, PathShape};
use crate::rc::Aspect;
use crate::text::{HAlign, Text, VAlign};
use crate::ticker::LabelFormat;

/// The face colour of outlines in the classic mode.
const CLASSIC_FACE: &str = "#bfd1d4";
/// The edge width of outlines in the classic mode, in points.
const CLASSIC_LINEWIDTH: f64 = 0.5;

/// A Sankey diagram under construction on an axes (matplotlib's `Sankey`).
#[derive(Debug)]
pub struct Sankey {
    axes: Axes,
    options: SankeyOptions,
    geometry: Geometry,
    diagrams: Vec<SankeyDiagram>,
    /// Tight `[xmin, xmax, ymin, ymax]` around the outlines and labels.
    extent: [f64; 4],
}

impl Sankey {
    /// An empty diagram on `axes`.
    ///
    /// # Errors
    ///
    /// When the gap, head angle or tolerance is negative, or the radius
    /// exceeds the gap.
    pub fn new(axes: &Axes, options: SankeyOptions) -> Result<Self, SankeyError> {
        if options.gap < 0.0 {
            return Err(SankeyError::NegativeGap);
        }
        if options.radius > options.gap {
            return Err(SankeyError::RadiusAboveGap);
        }
        if options.head_angle < 0.0 {
            return Err(SankeyError::NegativeHeadAngle);
        }
        if options.tolerance < 0.0 {
            return Err(SankeyError::NegativeTolerance);
        }
        let geometry = Geometry {
            gap: options.gap,
            radius: options.radius,
            shoulder: options.shoulder,
            offset: options.offset,
            pitch: (std::f64::consts::PI * (1.0 - options.head_angle / 180.0) / 2.0).tan(),
        };
        Ok(Self {
            axes: axes.clone(),
            options,
            geometry,
            diagrams: Vec::new(),
            extent: [
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ],
        })
    }

    /// Draws a subdiagram (matplotlib's `Sankey.add`).
    ///
    /// # Errors
    ///
    /// When the orientations, labels or path lengths do not match the
    /// flows, the trunk length is negative, the connection is invalid, or
    /// the quantity format accepts no number.
    pub fn add(&mut self, sub: Subdiagram) -> Result<&mut Self, SankeyError> {
        let n = sub.flows.len();
        let orientations = broadcast("orientations", &sub.orientations, n)?;
        let labels = broadcast("labels", &sub.labels, n)?;
        if sub.trunklength < 0.0 {
            return Err(SankeyError::NegativeTrunk);
        }
        if let Some(prior) = sub.prior {
            self.check_connection(prior, sub.connect, &sub.flows)?;
        }
        let placed = self.place_flows(&sub, &orientations)?;
        let outline = self.geometry.outline(&placed, sub.trunklength);
        let (codes, vertices): (Vec<_>, Vec<_>) = outline.steps.into_iter().unzip();
        let angles: Vec<Option<f64>> = placed
            .iter()
            .map(|f| f.direction.map(Direction::quarter_turns))
            .collect();
        let placement = Placement {
            angles,
            tips: outline.tips,
            labels: outline.label_locations,
            vertices,
        };
        let (placement, center) = match sub.prior {
            None => (placement.rotated(sub.rotation / 90.0), Point::ORIGIN),
            Some(prior) => self.connected(&placement, prior, sub.connect)?,
        };
        let text = self.centered_text(center, sub.patchlabel);
        let path =
            Path::new(placement.vertices.clone(), codes).expect("the outline starts with a move");
        let patch = self.axes.add_patch(PathShape(path));
        patch.update_props(self.outline_props(sub.props));
        let texts = sub
            .flows
            .iter()
            .zip(&placement.angles)
            .zip(labels)
            .zip(&placement.labels)
            .map(|(((&flow, angle), label), &location)| {
                let text = match (label, angle) {
                    (Some(label), Some(_)) => self.with_quantity(label, flow)?,
                    _ => String::new(),
                };
                Ok(self.centered_text(location, text))
            })
            .collect::<Result<Vec<_>, SankeyError>>()?;
        self.include(&placement.vertices);
        self.include(&placement.labels);
        self.diagrams.push(SankeyDiagram {
            patch,
            flows: sub.flows,
            angles: placement.angles,
            tips: placement.tips,
            text,
            texts,
        });
        Ok(self)
    }

    /// Fits the view around the subdiagrams with the margin and equal
    /// scaling (by changing the limits), and returns them (matplotlib's
    /// `Sankey.finish`).
    ///
    /// # Errors
    ///
    /// When no subdiagram was added or the margin is not finite, which
    /// leave no finite view; then the axes is unchanged.
    pub fn finish(&self) -> Result<&[SankeyDiagram], SankeyError> {
        let [x0, x1, y0, y1] = self.extent;
        let margin = self.options.margin;
        self.axes
            .set_xlim(x0 - margin, x1 + margin)
            .and_then(|axes| axes.set_ylim(y0 - margin, y1 + margin))
            .map_err(SankeyError::Limits)?;
        self.axes
            .set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
        self.axes
            .set_adjustable(Adjustable::Datalim)
            .expect("a Sankey diagram draws on rectilinear axes");
        Ok(&self.diagrams)
    }

    /// The subdiagrams drawn so far.
    pub fn diagrams(&self) -> &[SankeyDiagram] {
        &self.diagrams
    }

    /// The flows of `sub` as the outline places them: direction (none
    /// below the tolerance), scaled size and arrow length.
    fn place_flows(
        &self,
        sub: &Subdiagram,
        orientations: &[FlowOrientation],
    ) -> Result<Vec<PlacedFlow>, SankeyError> {
        let n = sub.flows.len();
        let tolerance = self.options.tolerance;
        let inputs: Vec<Option<bool>> = sub
            .flows
            .iter()
            .map(|&flow| {
                if flow >= tolerance {
                    Some(true)
                } else if flow <= -tolerance {
                    Some(false)
                } else {
                    None
                }
            })
            .collect();
        let directions: Vec<Option<Direction>> = orientations
            .iter()
            .zip(&inputs)
            .map(|(&orientation, &input)| input.map(|input| direction(orientation, input)))
            .collect();
        let scaled: Vec<f64> = sub.flows.iter().map(|f| self.options.scale * f).collect();
        let lengths = match &sub.pathlengths {
            PathLengths::PerFlow(lengths) if lengths.len() != n => {
                return Err(SankeyError::Count {
                    argument: "pathlengths",
                    len: lengths.len(),
                    flows: n,
                });
            }
            PathLengths::PerFlow(lengths) => lengths.clone(),
            PathLengths::Inner(length) => default_lengths(*length, &directions, &inputs, &scaled),
        };
        Ok((0..n)
            .map(|i| PlacedFlow {
                direction: directions[i],
                input: inputs[i] == Some(true),
                flow: scaled[i],
                length: lengths[i],
            })
            .collect())
    }

    /// Fails unless flow `connect.0` of diagram `prior` can meet flow
    /// `connect.1` of `flows`.
    fn check_connection(
        &self,
        prior: usize,
        (from, to): (usize, usize),
        flows: &[f64],
    ) -> Result<(), SankeyError> {
        let diagram = self.diagrams.get(prior).ok_or(SankeyError::UnknownPrior {
            prior,
            diagrams: self.diagrams.len(),
        })?;
        let unknown = |flow, flows: &[f64]| SankeyError::UnknownFlow {
            flow,
            flows: flows.len(),
        };
        let &prior_flow = diagram
            .flows
            .get(from)
            .ok_or_else(|| unknown(from, &diagram.flows))?;
        let &flow = flows.get(to).ok_or_else(|| unknown(to, flows))?;
        if diagram.angles[from].is_none() {
            return Err(SankeyError::HiddenFlow(from));
        }
        let error = prior_flow + flow;
        if error.abs() >= self.options.tolerance {
            return Err(SankeyError::Unbalanced {
                error,
                tolerance: self.options.tolerance,
            });
        }
        Ok(())
    }

    /// Turns and moves `placement` so that flow `to` meets flow `from` of
    /// diagram `prior`; returns it and the trunk centre.
    fn connected(
        &self,
        placement: &Placement,
        prior: usize,
        (from, to): (usize, usize),
    ) -> Result<(Placement, Point), SankeyError> {
        let diagram = &self.diagrams[prior];
        let (Some(target), Some(own)) = (diagram.angles[from], placement.angles[to]) else {
            return Err(SankeyError::HiddenFlow(to));
        };
        let rotation = target - own;
        let rotate = Affine2D::IDENTITY.rotate_deg(rotation * 90.0);
        let turned: Vec<Point> = placement
            .tips
            .iter()
            .map(|&p| rotate.transform_point(p))
            .collect();
        let offset = diagram.tips[from] - turned[to];
        let translate = Affine2D::translation(offset.x, offset.y);
        let shift = |points: &[Point]| -> Vec<Point> {
            points
                .iter()
                .map(|&p| translate.transform_point(rotate.transform_point(p)))
                .collect()
        };
        let moved = Placement {
            angles: turn(&placement.angles, rotation),
            tips: turned
                .iter()
                .map(|&p| translate.transform_point(p))
                .collect(),
            labels: shift(&placement.labels),
            vertices: shift(&placement.vertices),
        };
        Ok((moved, offset))
    }

    /// `label` followed by the flow's quantity, unless quantities are
    /// hidden.
    fn with_quantity(&self, mut label: String, flow: f64) -> Result<String, SankeyError> {
        let Some(unit) = &self.options.unit else {
            return Ok(label);
        };
        let quantity = match &self.options.format {
            LabelFormat::Template(_) => self.options.format.printf(flow.abs()),
            LabelFormat::Func(_) => self.options.format.printf(flow),
        }
        .map_err(SankeyError::Format)?;
        if !label.is_empty() {
            label.push('\n');
        }
        label.push_str(&quantity);
        label.push_str(unit);
        Ok(label)
    }

    /// The style of an outline: `props`, with the face colour defaulting
    /// to the next colour of the property cycle, or in the classic mode
    /// (`_internal.classic_mode`) to a grey blue with half-point edges.
    fn outline_props(&self, props: PatchProps) -> PatchProps {
        let classic = crate::rc::params().internal_classic_mode;
        let face = props.facecolor.clone().unwrap_or_else(|| {
            if classic {
                Color::from(CLASSIC_FACE)
            } else {
                self.axes.next_patch_color()
            }
        });
        PatchProps {
            facecolor: Some(face),
            linewidth: props.linewidth.or(classic.then_some(CLASSIC_LINEWIDTH)),
            ..props
        }
    }

    /// A text centred on `at` in data coordinates.
    fn centered_text(&self, at: Point, text: String) -> Text {
        let text = self.axes.text(at.x, at.y, text);
        text.set_halign(HAlign::Center).set_valign(VAlign::Center);
        text
    }

    /// Grows the extent to cover `points`.
    fn include(&mut self, points: &[Point]) {
        let [x0, x1, y0, y1] = &mut self.extent;
        for p in points {
            *x0 = x0.min(p.x);
            *x1 = x1.max(p.x);
            *y0 = y0.min(p.y);
            *y1 = y1.max(p.y);
        }
    }
}

/// A subdiagram's geometry in data coordinates.
struct Placement {
    angles: Vec<Option<f64>>,
    tips: Vec<Point>,
    labels: Vec<Point>,
    vertices: Vec<Point>,
}

impl Placement {
    /// The placement turned by `quarter_turns` (untouched when zero).
    fn rotated(self, quarter_turns: f64) -> Self {
        if quarter_turns == 0.0 {
            return self;
        }
        let rotate = Affine2D::IDENTITY.rotate_deg(quarter_turns * 90.0);
        let turn_all = |points: Vec<Point>| -> Vec<Point> {
            points
                .into_iter()
                .map(|p| rotate.transform_point(p))
                .collect()
        };
        Self {
            angles: turn(&self.angles, quarter_turns),
            tips: turn_all(self.tips),
            labels: turn_all(self.labels),
            vertices: turn_all(self.vertices),
        }
    }
}

/// Every drawn angle advanced by `quarter_turns`.
fn turn(angles: &[Option<f64>], quarter_turns: f64) -> Vec<Option<f64>> {
    angles
        .iter()
        .map(|a| a.map(|a| a + quarter_turns))
        .collect()
}

/// The arrow direction of an input (or output) on `orientation`'s side.
fn direction(orientation: FlowOrientation, input: bool) -> Direction {
    match (orientation, input) {
        (FlowOrientation::Side, _) => Direction::Right,
        (FlowOrientation::Top, true) | (FlowOrientation::Bottom, false) => Direction::Down,
        (FlowOrientation::Top, false) | (FlowOrientation::Bottom, true) => Direction::Up,
    }
}

/// matplotlib's lengths from one inner length: arrows on the top and
/// bottom grow from the middle outwards so that they nest; horizontal ones
/// get none (its lookup of the side length never matches).
fn default_lengths(
    inner: f64,
    directions: &[Option<Direction>],
    inputs: &[Option<bool>],
    flows: &[f64],
) -> Vec<f64> {
    let mut lengths = vec![0.0; flows.len()];
    let (mut upper_left, mut upper_right) = (inner, inner);
    for i in 0..flows.len() {
        match (directions[i], inputs[i]) {
            (Some(Direction::Down), Some(true)) => {
                lengths[i] = upper_left;
                upper_left += flows[i];
            }
            (Some(Direction::Up), Some(false)) => {
                lengths[i] = upper_right;
                upper_right -= flows[i];
            }
            _ => {}
        }
    }
    let (mut lower_left, mut lower_right) = (inner, inner);
    for i in (0..flows.len()).rev() {
        match (directions[i], inputs[i]) {
            (Some(Direction::Up), Some(true)) => {
                lengths[i] = lower_left;
                lower_left += flows[i];
            }
            (Some(Direction::Down), Some(false)) => {
                lengths[i] = lower_right;
                lower_right -= flows[i];
            }
            _ => {}
        }
    }
    lengths
}

/// `values` stretched to `n` flows (`np.broadcast_to`).
fn broadcast<T: Clone>(
    argument: &'static str,
    values: &[T],
    n: usize,
) -> Result<Vec<T>, SankeyError> {
    match values {
        [single] => Ok(vec![single.clone(); n]),
        _ if values.len() == n => Ok(values.to_vec()),
        _ => Err(SankeyError::Count {
            argument,
            len: values.len(),
            flows: n,
        }),
    }
}

#[cfg(test)]
mod tests;
