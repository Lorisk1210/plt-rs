//! The cases of `figures/cases_usetex.py` porting matplotlib's own usetex
//! tests (`test_usetex.py` and the usetex parts of `test_backend_pdf.py`,
//! `test_backend_svg.py`, `test_text.py` and `test_axes.py`).

use pltrs::axes::PieOptions;
use pltrs::patches::BoxStyle;
use pltrs::prelude::*;
use pltrs::text::{HAlign, TextBoxProps, TextProps, VAlign};
use pltrs::ticker::LabelFormat;

use super::usetex_case;
use crate::figures::FigureCase;

/// Texts drawn with `usetex=True` while `text.usetex` is off, with `rc`.
const fn artist_case(
    name: &'static str,
    rc: &'static [(&'static str, &'static str)],
    build: fn(&pltrs::figure::Figure) -> crate::figures::CaseResult,
) -> FigureCase {
    FigureCase::new(name, build)
        .sized(3.2, 2.0, 100.0)
        .with_rc(rc)
        .needing_tex()
}

pub(super) const CASES: &[FigureCase] = &[
    usetex_case("usetex/mpl_test_usetex", |fig| {
        // matplotlib's test_usetex.py::test_usetex (style mpl20).
        let ax = fig.subplot();
        let boxed = || -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
            Ok(Some(
                TextBoxProps::new("square,pad=0".parse::<BoxStyle>()?)
                    .with_patch(PatchProps::default().edgecolor("k").facecolor("none")),
            ))
        };
        let texts = [
            (
                0.2,
                0.7,
                concat!(
                    r"\LaTeX\ $\left[\int\limits_e^{-2e}",
                    r"\sqrt\frac{\log^3 x}{x}\,\mathrm{d}x \right\}$"
                ),
            ),
            (0.2, 0.3, "lg"),
            (0.4, 0.3, r"$\frac{1}{2}\pi$"),
            (0.6, 0.3, "$p^{3^A}$"),
            (0.8, 0.3, "$p_{3_2}$"),
        ];
        for (x, y, text) in texts {
            ax.text(x, y, text)
                .set_valign(VAlign::Baseline)
                .set_fontsize(24.0)
                .set_bbox(boxed()?);
        }
        for x in [0.2, 0.4, 0.6, 0.8] {
            ax.axvline(x, LineProps::default());
        }
        for y in [0.3, 0.7] {
            ax.axhline(y, LineProps::default());
        }
        ax.set_axis_off();
        Ok(())
    })
    .sized(6.4, 4.8, 100.0),
    usetex_case("usetex/empty", |fig| {
        // matplotlib's test_empty: a comment typesets nothing.
        fig.text(0.5, 0.5, "% a comment");
        Ok(())
    })
    .sized(2.0, 1.0, 100.0),
    usetex_case("usetex/unicode_minus", |fig| {
        // matplotlib's test_unicode_minus: both texts draw the same minus.
        fig.text(0.3, 0.5, "$-$");
        fig.text(0.7, 0.5, "\u{2212}");
        Ok(())
    })
    .sized(2.0, 1.0, 100.0),
    artist_case(
        "usetex/mathdefault",
        &[
            ("text.usetex", "False"),
            ("axes.formatter.use_mathtext", "True"),
        ],
        |fig| {
            // matplotlib's test_mathdefault: \mathdefault from the tick
            // formatter, usetex switched on before drawing.
            fig.subplot().set_xlim(-1.0, 1.0)?;
            pltrs::rc::set("text.usetex", "True")?;
            Ok(())
        },
    )
    .sized(3.2, 2.4, 100.0),
    usetex_case("usetex/eqnarray", |fig| {
        // matplotlib's test_multiline_eqnarray.
        let text = concat!(
            r"\begin{eqnarray*}",
            r"foo\\",
            r"bar\\",
            r"baz\\",
            r"\end{eqnarray*}"
        );
        fig.text(0.5, 0.5, text)
            .set_usetex(true)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
        Ok(())
    })
    .sized(1.0, 1.0, 100.0),
    artist_case(
        "usetex/pkg_already_loaded",
        &[
            ("text.usetex", "False"),
            ("text.latex.preamble", r"\usepackage[full]{textcomp}"),
        ],
        |fig| {
            // matplotlib's test_latex_pkg_already_loaded.
            fig.text(0.5, 0.5, "hello, world")
                .set_usetex(true)
                .set_halign(HAlign::Center);
            Ok(())
        },
    ),
    artist_case(
        "usetex/underscore_preamble",
        &[
            ("text.usetex", "False"),
            ("text.latex.preamble", r"\usepackage{underscore}"),
        ],
        |fig| {
            fig.text(0.5, 0.5, "hello_world")
                .set_usetex(true)
                .set_halign(HAlign::Center);
            Ok(())
        },
    ),
    artist_case("usetex/single_artist", &[("text.usetex", "False")], |fig| {
        // matplotlib's test_text.py::test_single_artist_usetex.
        fig.text(0.3, 0.5, r"$\frac12$").set_usetex(true);
        fig.text(0.6, 0.5, r"$\frac{1}{2}$");
        Ok(())
    }),
    usetex_case("usetex/pdf_kerning", |fig| {
        // matplotlib's test_backend_pdf.py usetex texts at 50 pt.
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
        ax.set_axis_off();
        ax.text(0.05, 0.55, "BHTem fi ffl 1234")
            .set_usetex(true)
            .set_fontsize(50.0);
        ax.text(0.05, 0.1, r"\textsf{fi ffl 1234}")
            .set_usetex(true)
            .set_fontsize(30.0);
        Ok(())
    })
    .sized(6.4, 2.4, 100.0),
    usetex_case("usetex/textwon", |fig| {
        // matplotlib's test_backend_svg.py::test_unicode_won.
        fig.text(0.5, 0.5, r"\textwon").set_usetex(true);
        Ok(())
    })
    .sized(2.0, 1.0, 100.0),
    artist_case("usetex/pie_textprops", &[("text.usetex", "False")], |fig| {
        // matplotlib's test_axes.py::test_pie_textprops with usetex.
        let options = PieOptions {
            autopct: Some(LabelFormat::Template(r"%1.0f\%%".into())),
            textprops: TextProps::default().usetex(true),
            ..PieOptions::default()
        };
        fig.subplot().pie([23.0, 34.0, 45.0], options)?;
        Ok(())
    })
    .sized(3.2, 2.4, 100.0),
];
