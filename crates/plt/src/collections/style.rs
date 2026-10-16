//! Colours, widths and dash patterns of collection items after defaults
//! and colour mapping.

use plt_core::Rgba;
use plt_render::Dashes;

use super::{CollectionArtist, CollectionKind, EdgeColors};
use crate::artist::Artist;
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::RcParams;

/// Face and edge colours after defaults, colour mapping and alpha
/// (matplotlib's `_set_facecolor`, `_set_edgecolor` and
/// `update_scalarmappable`).
pub(super) fn resolve_colors(c: &CollectionArtist) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
    let rc = c.rc();
    let alpha = c.base().alpha;
    // A lone "none" means no colours at all (matplotlib's `to_rgba_array`).
    let resolve = |colors: &[Color]| -> Result<Vec<Rgba>, String> {
        if let [only] = colors
            && only.is_none()
        {
            return Ok(Vec::new());
        }
        colors
            .iter()
            .map(|color| color.resolve_alpha(alpha).map_err(|e| e.to_string()))
            .collect()
    };
    let face_spec = c.facecolors.clone().unwrap_or_else(|| {
        vec![if c.kind == CollectionKind::Line {
            Color::none()
        } else {
            rc.patch_facecolor.clone()
        }]
    });
    let face_is_none = face_spec.len() == 1 && face_spec[0].is_none();
    let edge_default = c.kind == CollectionKind::Line;

    let mapped = c.mapping.as_ref().map(|m| m.colors(alpha)).transpose()?;
    let face_mapped = mapped.is_some() && !face_is_none;
    let edge_mapped = mapped.is_some()
        && if face_is_none {
            matches!(c.edgecolors, EdgeColors::Default)
        } else {
            matches!(c.edgecolors, EdgeColors::Face)
        };

    let faces = match (&mapped, face_mapped) {
        (Some(colors), true) => colors.clone(),
        _ => resolve(&face_spec)?,
    };
    let edges = match (&mapped, edge_mapped, &c.edgecolors) {
        (Some(colors), true, _) => colors.clone(),
        (_, _, EdgeColors::Face) => faces.clone(),
        (_, _, EdgeColors::Colors(colors)) => resolve(colors)?,
        (_, _, EdgeColors::Default) => {
            if rc.patch_force_edgecolor || edge_default || face_is_none {
                let default = if c.kind == CollectionKind::Line {
                    &rc.lines_color
                } else {
                    &rc.patch_edgecolor
                };
                resolve(std::slice::from_ref(default))?
            } else {
                Vec::new()
            }
        }
    };
    Ok((faces, edges))
}

/// Pairs line widths with dash patterns (cycling both to their least
/// common multiple) and scales each pattern by its width (matplotlib's
/// `Collection._bcast_lwls`).
pub(super) fn broadcast_widths_and_dashes(
    widths: &[f64],
    styles: &[LineStyle],
    rc: &RcParams,
) -> (Vec<f64>, Vec<Dashes>) {
    let styles = if styles.is_empty() {
        &[LineStyle::Solid][..]
    } else {
        styles
    };
    let widths = if widths.is_empty() {
        &[0.0][..]
    } else {
        widths
    };
    let n = lcm(widths.len(), styles.len());
    let paired_widths: Vec<f64> = (0..n).map(|i| widths[i % widths.len()]).collect();
    let dashes = (0..n)
        .map(|i| styles[i % styles.len()].scaled_dashes(paired_widths[i], rc))
        .collect();
    (paired_widths, dashes)
}

fn lcm(a: usize, b: usize) -> usize {
    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    a / gcd(a, b) * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_and_dashes_cycle_to_common_length() {
        let rc = RcParams::matplotlib();
        let (widths, dashes) = broadcast_widths_and_dashes(
            &[1.0, 2.0],
            &[LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted],
            &rc,
        );
        assert_eq!(widths.len(), 6);
        assert_eq!(widths, [1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
        assert_eq!(dashes[1].pattern, vec![3.7 * 2.0, 1.6 * 2.0]);
    }

    #[test]
    fn least_common_multiple() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(1, 1), 1);
    }
}
