//! Title and label cases of `figures/cases_axes.py`.

use plt::axes::{LabelLoc, TitleLocation};
use plt::fonts::{FontSize, FontWeight, RelativeSize};
use plt::gridspec::SubplotOverrides;

use super::line;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/title_locations", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_title_at(TitleLocation::Left, "Left title");
        ax.set_title("Center title");
        ax.set_title_at(TitleLocation::Right, "Right title");
        Ok(())
    }),
    FigureCase::new("axes/title_pad_y_fontdict", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_title("Padded bold title")
            .set_fontsize(16.0)
            .set_weight(FontWeight::BOLD)
            .set_color("navy");
        ax.set_title_pad(20.0);
        // matplotlib's second set_title resets the pad to axes.titlepad.
        ax.set_title_at(TitleLocation::Left, "low")
            .set_fontsize(FontSize::Relative(RelativeSize::Small));
        ax.set_title_y(TitleLocation::Left, 0.9);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/suptitle_supxlabel_supylabel", |fig| {
        for ax in fig.subplots(1, 2)?.as_slice() {
            line(ax)?;
        }
        fig.suptitle("Figure title");
        fig.supxlabel("shared x label");
        fig.supylabel("shared y label");
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("axes/xlabel_ylabel_default", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_xlabel("time [s]").set_ylabel("amplitude");
        Ok(())
    }),
    FigureCase::new("axes/xlabel_ylabel_loc_and_pad", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(SubplotOverrides {
            left: Some(0.18),
            ..SubplotOverrides::default()
        })?;
        line(&ax)?;
        ax.set_xlabel("x at right")
            .xaxis()
            .set_label_loc(LabelLoc::End);
        ax.set_ylabel("y at top")
            .yaxis()
            .set_label_loc(LabelLoc::End)
            .set_label_pad(12.0)
            .set_label_color("tab:red")
            .set_label_fontsize(FontSize::Points(14.0));
        Ok(())
    }),
    FigureCase::new("axes/mathtext_labels", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_title(r"$\alpha^2 + \beta_i = \frac{1}{2}\sqrt{x}$");
        ax.set_xlabel(r"$\omega t$ [rad]")
            .set_ylabel(r"$\sin(\omega t)$");
        Ok(())
    }),
];
