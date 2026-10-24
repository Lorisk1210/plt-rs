//! Figure-level texts, labels and colours: `suptitle` with font
//! properties, the super-label getters, removing them, subfigure
//! facecolours and figure labels.

use plt::artist::ArtistHandle;
use plt::colors::Color;
use plt::figure::{Figure, SubfigureOptions};

use super::super::mpl_tests::in_test_style;
use super::super::text_metrics::font_properties;
use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Outcome, ProbeError};

/// `fig.suptitle(..., fontproperties=FontProperties(**fontproperties))`:
/// the title's size and numeric weight.
pub(super) fn suptitle(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let font = font_properties(args.json("fontproperties")?)?;
        let fig = Figure::new();
        fig.subplot();
        let title = fig.suptitle("fontprops title");
        title.set_font(font);
        let font = title.font();
        Ok(object([
            ("fontsize", Json::from(font.size)),
            ("weight", Json::from(f64::from(font.weight.value()))),
        ]))
    })
}

/// The facecolours of two subfigures, the second set to white.
pub(super) fn subfigure_facecolors(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::with_size(4.0, 3.0, plt::rc::params().figure_dpi);
        let parts = subfigures(&fig, 1, 2)?;
        let (left, right) = (&parts[0], &parts[1]);
        right.set_facecolor(plt_core::Rgba::WHITE);
        left.subplot();
        right.subplot();
        fig.suptitle("This is a visible suptitle.");
        let resolved = |color: Color| color.resolve().map_err(ProbeError::rejected);
        Ok(Json::Array(vec![
            rgba(resolved(left.facecolor())?),
            rgba(resolved(right.facecolor())?),
        ]))
    })
}

/// The super-label getters of a new figure, then after setting each.
pub(super) fn get_suptitles(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot();
        let mut out = vec![getters(&fig)];
        fig.suptitle("suptitle");
        out.push(getters(&fig));
        fig.supxlabel("supxlabel");
        out.push(getters(&fig));
        fig.supylabel("supylabel");
        out.push(getters(&fig));
        Ok(Json::Array(out))
    })
}

/// The number of figure texts as the three super-labels are removed.
pub(super) fn remove_suptitles(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let texts = [
            fig.suptitle("suptitle"),
            fig.supxlabel("supxlabel"),
            fig.supylabel("supylabel"),
        ];
        let mut counts = vec![fig.texts().len()];
        for text in &texts {
            text.remove();
            counts.push(fig.texts().len());
        }
        Ok(object([
            (
                "texts",
                Json::from(counts.into_iter().map(|n| n as f64).collect::<Vec<_>>()),
            ),
            ("getters", getters(&fig)),
        ]))
    })
}

/// The labels of a figure and of its single subfigure.
pub(super) fn labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.set_label("whole Figure");
        let sub = subfigures(&fig, 1, 1)?;
        sub[0].set_label("sub figure");
        Ok(Json::from(vec![fig.label(), sub[0].label()]))
    })
}

/// `[suptitle, supxlabel, supylabel]`.
fn getters(fig: &Figure) -> Json {
    Json::from(vec![
        fig.suptitle_text(),
        fig.supxlabel_text(),
        fig.supylabel_text(),
    ])
}

/// `fig.subfigures(nrows, ncols)` in row order.
pub(super) fn subfigures(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
) -> Result<Vec<Figure>, ProbeError> {
    fig.subfigures(nrows, ncols, SubfigureOptions::default())
        .map(|grid| grid.as_slice().to_vec())
        .map_err(ProbeError::rejected)
}
