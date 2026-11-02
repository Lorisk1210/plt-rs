"""Rules for transforms and scale."""
from rules_base import cov, diff, na, miss, ALIAS, CHANGE_TRACKING, TRANSFORM_META

T = r"matplotlib\.transforms:"
S = r"matplotlib\.scale:"

RULES = [
    (T + r"Affine2D\.clear", diff("Affine2D::identity", "an Affine2D is a value: start from the identity")),
    (T + r".*\.frozen", na("frozen transforms: pltrs transforms are resolved values already (conventions)")),
    (T + r".*\.(invalidate|set_children)", CHANGE_TRACKING),
    (T + r".*\.depth", TRANSFORM_META),
    (T + r".*\.(transform_affine|get_affine)", cov("ResolvedTransform::affine_part / ResolvedTransform::affine")),
    (T + r".*\.transform_non_affine", cov("ResolvedTransform::transform_point_non_affine")),
    (T + r".*\.(transform_path_affine|transform_path_non_affine)", cov("ResolvedTransform::split_path")),
    (T + r".*\.transform_path", cov("ResolvedTransform::transform_path")),
    (T + r".*\.transform", cov("ResolvedTransform::transform_points / Affine2D::transform_point")),
    (T + r".*\.get_matrix", cov("Affine2D::matrix")),
    (T + r"Transform\.contains_branch_seperately", ALIAS),
    (T + r"Bbox\.unit", diff("Bbox::from_extents", "Bbox::from_extents(0.0, 0.0, 1.0, 1.0)")),
    (T + r"Bbox\.null", diff("TrackedBbox::new", "a tracked box starts null")),
    (T + r"(Bbox|BboxBase)\.intervalx", cov("Bbox::interval_x")),
    (T + r"(Bbox|BboxBase)\.intervaly", cov("Bbox::interval_y")),
    (T + r"Bbox\.(minposx|minposy)", cov("TrackedBbox::minpos")),
    (T + r"(Bbox|BboxBase|LockableBbox|TransformedBbox)\.get_points", cov("Bbox::p0 / Bbox::p1")),
    (T + r"Bbox\.set_points", diff("Bbox::from_points", "a Bbox is a value: make a new one")),
    (T + r"Bbox\.mutatedx", cov("TrackedBbox::mutated_x")),
    (T + r"Bbox\.mutatedy", cov("TrackedBbox::mutated_y")),
    (T + r"BboxBase\.min", cov("Bbox::p0")),
    (T + r"BboxBase\.max", cov("Bbox::p1")),
    (T + r"BboxBase\.size", cov("Bbox::width / Bbox::height")),
    (T + r"BboxBase\.containsx", cov("Bbox::contains_x")),
    (T + r"BboxBase\.containsy", cov("Bbox::contains_y")),
    (T + r"BboxBase\.fully_containsx", cov("Bbox::fully_contains_x")),
    (T + r"BboxBase\.fully_containsy", cov("Bbox::fully_contains_y")),
    (T + r"LockableBbox", miss("a box following another with some edges locked")),
    (T + r"LockableBbox\..*", miss("a box following another with some edges locked")),
    (T + r"TransformedPath\.(get_transformed_path_and_affine|get_fully_transformed_path)", cov("ResolvedTransform::split_path / ResolvedTransform::transform_path")),
    (S + r".*\.get_transform", diff("Scale::forward / Scale::inverse", "a scale is its own transform")),
    (S + r".*\.set_default_locators_and_formatters", cov("Scale::default_tickers (AxisTickers)")),
    (S + r".*\.transform_non_affine", cov("forward of the scale transform")),
    (S + r".*\.inverted", cov("inverse of the scale transform")),
    (S + r"SymmetricalLogScale\.(base|linscale|linthresh)", cov("SymmetricalLogScale::transform (SymmetricalLogTransform::base / linscale / linthresh)")),
]
