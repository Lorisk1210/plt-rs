//! The transform trees of `TestBasicTransform`: three affine maps
//! (`ta1`, `ta2`, `ta3`) and three distinct non-affine translations
//! (`tn1`, `tn2`, `tn3`), combined with `+` and blends.

use std::f64::consts::FRAC_PI_2;

use pltrs::transforms::{Affine2D, Point, Transform};

use super::{NonAffine, resolve};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The six named transforms of the test's `setup_method`.
struct Leaves {
    affine: [Transform; 3],
    non_affine: [Transform; 3],
}

impl Leaves {
    fn new() -> Self {
        let shift = || Transform::custom(NonAffine(Affine2D::translation(1.0, 2.0)));
        Self {
            affine: [
                Transform::Affine(Affine2D::IDENTITY.rotate(FRAC_PI_2)),
                Transform::Affine(Affine2D::translation(10.0, 0.0)),
                Transform::Affine(Affine2D::scaling(1.0, 2.0)),
            ],
            non_affine: [shift(), shift(), shift()],
        }
    }

    /// The transform a tree describes: a leaf name, `["+", a, b]` or
    /// `["blend", x, y]`.
    fn tree(&self, spec: &Json) -> Result<Transform, ProbeError> {
        let bad = || ProbeError::BadArgs(format!("not a transform tree: {spec}"));
        if let Some(name) = spec.as_str() {
            let index = |digit: &str| match digit {
                "1" => Some(0),
                "2" => Some(1),
                "3" => Some(2),
                _ => None,
            };
            let leaf = match name.split_at_checked(2) {
                Some(("ta", digit)) => index(digit).map(|i| &self.affine[i]),
                Some(("tn", digit)) => index(digit).map(|i| &self.non_affine[i]),
                _ => None,
            };
            return leaf.cloned().ok_or_else(bad);
        }
        match spec.as_array() {
            Some([kind, a, b]) => {
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
}

/// The `tree` argument and, with `branch`, the `branch` one, built over
/// the same leaves.
fn tree_and_branch(args: Args<'_>) -> Result<(Transform, Transform), ProbeError> {
    let leaves = Leaves::new();
    Ok((
        leaves.tree(args.json("tree")?)?,
        leaves.tree(args.json("branch")?)?,
    ))
}

fn tree(args: Args<'_>) -> Result<Transform, ProbeError> {
    Leaves::new().tree(args.json("tree")?)
}

/// `tree.contains_branch(branch)`.
pub(super) fn contains_branch(args: Args<'_>) -> Outcome {
    let (tree, branch) = tree_and_branch(args)?;
    Ok(Json::from(tree.contains_branch(&branch)))
}

/// `tree.contains_branch_seperately(branch)` as `[x, y]`.
pub(super) fn contains_branch_separately(args: Args<'_>) -> Outcome {
    let (tree, branch) = tree_and_branch(args)?;
    let (x, y) = tree.contains_branch_separately(&branch);
    Ok(Json::Array(vec![Json::from(x), Json::from(y)]))
}

/// The `points` argument mapped by `map`.
fn mapped(args: Args<'_>, map: impl Fn(Point) -> Point) -> Outcome {
    let points = encode::parse_points(args.json("points")?)?;
    let mapped: Vec<Point> = points.into_iter().map(map).collect();
    Ok(encode::points(&mapped))
}

/// `tree.transform_non_affine(points)`.
pub(super) fn transform_non_affine(args: Args<'_>) -> Outcome {
    let resolved = resolve(&tree(args)?)?;
    mapped(args, |p| resolved.transform_point_non_affine(p))
}

/// `tree.transform(points)`.
pub(super) fn transform(args: Args<'_>) -> Outcome {
    let resolved = resolve(&tree(args)?)?;
    mapped(args, |p| resolved.transform_point(p))
}

/// `tree.get_affine().get_matrix()`.
pub(super) fn get_affine(args: Args<'_>) -> Outcome {
    Ok(encode::matrix(&resolve(&tree(args)?)?.affine_part()))
}
