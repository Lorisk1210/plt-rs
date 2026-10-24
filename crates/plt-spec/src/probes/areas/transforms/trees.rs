//! Transform expressions on a figure with two axes: which transforms a
//! transform ends with (`contains_branch`, `contains_branch_separately`)
//! and boxes mapped through a data transform (`transform_bbox`).

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::figure::Figure;
use plt::transforms::{Affine2D, Transform};

use super::bbox_arg;
use crate::json::Json;
use crate::probes::areas::text_metrics::with_rc;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The figure and the two axes (`add_subplot(1, 2, 1)` and `(1, 2, 2)`)
/// the trees refer to.
struct Scene {
    figure: Figure,
    axes: [Axes; 2],
}

impl Scene {
    fn new() -> Result<Self, ProbeError> {
        let figure = Figure::new();
        let cell = |index| {
            figure
                .add_subplot(1, 2, index)
                .map_err(ProbeError::rejected)
        };
        let axes = [cell(1)?, cell(2)?];
        Ok(Self { figure, axes })
    }

    /// The transform a tree describes (see the module docs of
    /// `test_transforms.py`).
    fn tree(&self, spec: &Json) -> Result<Transform, ProbeError> {
        let bad = || ProbeError::BadArgs(format!("not a transform tree: {spec}"));
        if let Some(name) = spec.as_str() {
            let [ax, ax2] = &self.axes;
            return Ok(match name {
                "display" => Transform::Display,
                "data" => ax.trans_data(),
                "axes" => ax.trans_axes(),
                "data2" => ax2.trans_data(),
                "figure" => Transform::Figure(self.figure.id()),
                "inches" => Transform::Inches,
                _ => return Err(bad()),
            });
        }
        match spec.as_array().ok_or_else(bad)? {
            [kind, ops] if kind.as_str() == Some("affine") => Ok(Transform::Affine(
                encode::apply_affine_ops(Affine2D::IDENTITY, ops)?,
            )),
            [kind, a, b] => {
                let (a, b) = (self.tree(a)?, self.tree(b)?);
                match kind.as_str() {
                    Some("+") => Ok(a.then(b)),
                    Some("blend") => Ok(Transform::blended(a, b)),
                    _ => Err(bad()),
                }
            }
            _ => Err(bad()),
        }
    }

    /// The `tree` and `branch` arguments.
    fn tree_and_branch(&self, args: Args<'_>) -> Result<(Transform, Transform), ProbeError> {
        Ok((
            self.tree(args.json("tree")?)?,
            self.tree(args.json("branch")?)?,
        ))
    }
}

/// `tree.contains_branch(branch)`.
pub(super) fn contains_branch(args: Args<'_>) -> Outcome {
    let (tree, branch) = Scene::new()?.tree_and_branch(args)?;
    Ok(Json::from(tree.contains_branch(&branch)))
}

/// `tree.contains_branch_separately(branch)` as `[x, y]`.
pub(super) fn contains_branch_separately(args: Args<'_>) -> Outcome {
    let (tree, branch) = Scene::new()?.tree_and_branch(args)?;
    let (x, y) = tree.contains_branch_separately(&branch);
    Ok(Json::Array(vec![Json::from(x), Json::from(y)]))
}

/// `ax.transData.transform_bbox(bbox)` after setting the scales and
/// limits.
pub(super) fn transform_bbox(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let figure = Figure::new();
        let ax = figure.subplot();
        ax.set_xscale(args.str("xscale")?)
            .map_err(ProbeError::rejected)?;
        ax.set_yscale(args.str("yscale")?)
            .map_err(ProbeError::rejected)?;
        let ((x0, x1), (y0, y1)) = (limits(args, "xlim")?, limits(args, "ylim")?);
        ax.set_xlim(x0, x1).map_err(ProbeError::rejected)?;
        ax.set_ylim(y0, y1).map_err(ProbeError::rejected)?;
        let data = figure
            .resolve_transform(&ax.trans_data())
            .map_err(ProbeError::rejected)?;
        Ok(encode::bbox(&data.transform_bbox(&bbox_arg(args, "bbox")?)))
    })
}

/// A `[low, high]` argument.
fn limits(args: Args<'_>, key: &str) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec(key)?[..] {
        [low, high] => Ok((low, high)),
        _ => Err(ProbeError::BadArgs(format!("{key} needs two limits"))),
    }
}
