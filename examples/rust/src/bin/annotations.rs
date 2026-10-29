//! Annotations and arrows: a year of a share price with its high, its
//! sharpest fall and a summary box pointed out.

use plt::layout::LayoutEngine;
use plt::patches::BoxStyle;
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, TextBoxProps, VAlign};
use plt_examples::Samples;

/// The index of the largest of `values`.
fn argmax(values: impl IntoIterator<Item = f64>) -> usize {
    let indexed = values.into_iter().enumerate();
    indexed
        .fold((0, f64::NEG_INFINITY), |best, (i, v)| {
            if v > best.1 { (i, v) } else { best }
        })
        .0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut returns = Samples::new(21).normal(250, 0.0012, 0.014);
    returns[170] -= 0.11; // a profit warning
    let price: Vec<f64> = returns
        .iter()
        .scan(0.0, |total, r| {
            *total += r;
            Some(100.0 * total.exp())
        })
        .collect();
    let day = arange(0.0, 250.0, 1.0);
    let high = argmax(price.iter().copied());
    let fall = argmax(price.windows(2).map(|pair| pair[0] - pair[1])) + 1;

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.plot_with(&day, &price, LineProps::default().linewidth(1.5))?;
    let curved = FancyArrowProps {
        connectionstyle: "arc3,rad=0.3".parse()?,
        ..FancyArrowProps::new("->".parse()?)
    };
    let options = AnnotateOptions {
        xytext: Some((25.0, 12.0).into()),
        textcoords: Some("offset points".parse()?),
        arrowprops: Some(ArrowProps::Fancy(curved)),
        ..AnnotateOptions::default()
    };
    ax.annotate(
        format!("high: {:.1}", price[high]),
        (day[high], price[high]),
        options,
    )?;
    let red = FancyArrowProps {
        patch: PatchProps::default().color("tab:red").linewidth(1.5),
        ..FancyArrowProps::new("-|>".parse()?)
    };
    let options = AnnotateOptions {
        xytext: Some((-120.0, -8.0).into()),
        textcoords: Some("offset points".parse()?),
        arrowprops: Some(ArrowProps::Fancy(red)),
        ..AnnotateOptions::default()
    };
    let warning = ax.annotate("profit warning", (day[fall], price[fall]), options)?;
    warning.set_color("tab:red");
    warning.set_bbox(Some(
        TextBoxProps::new("round,pad=0.4".parse::<BoxStyle>()?).with_patch(
            PatchProps::default()
                .facecolor("white")
                .edgecolor("tab:red"),
        ),
    ));
    let (first, last) = (price[0], price[price.len() - 1]);
    let change = 100.0 * (last / first - 1.0);
    let summary = ax.text(0.03, 0.95, format!("close {last:.1}\nyear {change:+.1}%"));
    summary
        .set_transform(ax.trans_axes())
        .set_valign(VAlign::Top);
    summary.set_bbox(Some(
        TextBoxProps::new("round,pad=0.5".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().facecolor("0.96").edgecolor("0.8")),
    ));
    ax.set_title("Share price over one year");
    ax.set_xlabel("trading day").set_ylabel("price (€)");
    ax.set_xlim(0.0, 249.0)?.set_ylim(75.0, 115.0)?;

    fig.savefig("annotations.png")?;
    Ok(())
}
