//! Wrapping text to the box of its figure (matplotlib's
//! `Text._get_wrapped_text`).

use plt_core::{Bbox, Point};
use plt_text::HAlign;

/// Distance from `(x0, y0)` to the edge of `figure` along the direction
/// `rotation` degrees counter-clockwise from the x axis
/// (`Text._get_dist_to_box`).
fn dist_to_box(rotation: f64, x0: f64, y0: f64, figure: &Bbox) -> f64 {
    let cos_deg = |deg: f64| deg.to_radians().cos();
    let (h1, h2) = if rotation > 270.0 {
        let quad = rotation - 270.0;
        (
            (y0 - figure.y0()) / cos_deg(quad),
            (figure.x1() - x0) / cos_deg(90.0 - quad),
        )
    } else if rotation > 180.0 {
        let quad = rotation - 180.0;
        (
            (x0 - figure.x0()) / cos_deg(quad),
            (y0 - figure.y0()) / cos_deg(90.0 - quad),
        )
    } else if rotation > 90.0 {
        let quad = rotation - 90.0;
        (
            (figure.y1() - y0) / cos_deg(quad),
            (x0 - figure.x0()) / cos_deg(90.0 - quad),
        )
    } else {
        (
            (figure.x1() - x0) / cos_deg(rotation),
            (figure.y1() - y0) / cos_deg(90.0 - rotation),
        )
    };
    h1.min(h2)
}

/// The width available to a wrapped line (`Text._get_wrap_line_width`):
/// the distance from the anchor to the figure edge ahead of the text when
/// left-aligned, behind it when right-aligned, and twice the smaller of
/// both when centred. `angle` is the text rotation in `[0, 360)` degrees.
pub(crate) fn wrap_width(anchor: Point, figure: &Bbox, angle: f64, halign: HAlign) -> f64 {
    let ahead = dist_to_box(angle, anchor.x, anchor.y, figure);
    let behind = dist_to_box((180.0 + angle) % 360.0, anchor.x, anchor.y, figure);
    match halign {
        HAlign::Left => ahead,
        HAlign::Right => behind,
        HAlign::Center => 2.0 * ahead.min(behind),
    }
}

/// Breaks every line of `text` between words (at single spaces) so that
/// each wrapped line is at most `line_width` wide, as measured by `width`.
/// A line is only measured from its second word on, so a single word too
/// wide for the box stays on its own line; explicit newlines always break.
pub(crate) fn wrap_text<E>(
    text: &str,
    line_width: f64,
    mut width: impl FnMut(&str) -> Result<f64, E>,
) -> Result<String, E> {
    let mut wrapped: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let mut words: &[&str] = &line.split(' ').collect::<Vec<_>>();
        while let Some((first, rest)) = words.split_first() {
            if rest.is_empty() {
                wrapped.push((*first).to_owned());
                break;
            }
            let mut taken = words.len();
            for end in 2..=words.len() {
                if width(&words[..end].join(" "))? > line_width {
                    taken = end - 1;
                    break;
                }
            }
            wrapped.push(words[..taken].join(" "));
            words = &words[taken..];
        }
    }
    Ok(wrapped.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps with one unit of width per character.
    fn wrap(text: &str, line_width: f64) -> String {
        wrap_text(text, line_width, |line| {
            Ok::<_, ()>(line.chars().count() as f64)
        })
        .unwrap()
    }

    #[test]
    fn widths_toward_the_figure_edges() {
        let figure = Bbox::from_bounds(0.0, 0.0, 640.0, 480.0);
        let anchor = Point::new(160.0, 240.0);
        assert_eq!(wrap_width(anchor, &figure, 0.0, HAlign::Left), 480.0);
        assert_eq!(wrap_width(anchor, &figure, 0.0, HAlign::Right), 160.0);
        assert_eq!(wrap_width(anchor, &figure, 0.0, HAlign::Center), 320.0);
        let up = wrap_width(anchor, &figure, 90.0, HAlign::Left);
        assert!((up - 240.0).abs() < 1e-9);
    }

    #[test]
    fn breaks_between_words() {
        assert_eq!(wrap("aa bb cc dd", 5.0), "aa bb\ncc dd");
        assert_eq!(wrap("aaaaaaa b", 5.0), "aaaaaaa\nb");
        assert_eq!(wrap("a  b", 1.0), "a\n\nb");
        assert_eq!(wrap("ab\ncd ef", 10.0), "ab\ncd ef");
        assert_eq!(wrap("", 10.0), "");
    }

    #[test]
    fn measurement_errors_propagate() {
        assert_eq!(wrap_text("a b", 10.0, |_| Err::<f64, _>("bad")), Err("bad"));
    }
}
