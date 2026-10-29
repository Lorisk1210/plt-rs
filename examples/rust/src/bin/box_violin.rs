//! Box plots and violin plots: the exam scores of four classes taught four
//! ways, summarised side by side on a shared axis.

use plt::axes::{
    BoxArtist, BoxProps, BoxplotOptions, BxpOptions, ViolinDrawOptions, ViolinplotOptions,
};
use plt::figure::Share;
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt_examples::Samples;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = Samples::new(11);
    let methods = ["lecture", "seminar", "project", "online"];
    let spreads = [(68.0, 11.0), (74.0, 8.0), (77.0, 13.0), (63.0, 15.0)];
    let scores: Vec<Vec<f64>> = spreads
        .iter()
        .map(|&(mean, sd)| {
            let draws = rng.normal(120, mean, sd);
            draws.into_iter().map(|s| s.clamp(0.0, 100.0)).collect()
        })
        .collect();
    let colors = ["C0", "C1", "C2", "C3"];

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
    let options = BoxplotOptions {
        tick_labels: Some(methods.map(String::from).to_vec()),
        draw: BxpOptions {
            boxes: Some(BoxProps::Patch(PatchProps::default())),
            widths: vec![0.55],
            medianprops: LineProps::default().color("white").linewidth(1.5),
            ..BxpOptions::default()
        },
        ..BoxplotOptions::default()
    };
    let boxes = left.boxplot(&scores, options)?;
    for (artist, color) in boxes.boxes.iter().zip(colors) {
        if let BoxArtist::Patch(patch) = artist {
            patch.set_facecolor(color);
        }
    }
    left.set_title("Box plot");
    left.set_ylabel("score");

    let options = ViolinplotOptions {
        draw: ViolinDrawOptions {
            showmedians: true,
            facecolors: colors.map(Color::from).to_vec(),
            linecolors: vec!["0.35".into()],
            ..ViolinDrawOptions::default()
        },
        ..ViolinplotOptions::default()
    };
    right.violinplot(&scores, options)?;
    right
        .set_xticks([1.0, 2.0, 3.0, 4.0])
        .set_xticklabels(methods)?;
    right.set_title("Violin plot");
    fig.suptitle("Exam scores by teaching method");

    fig.savefig("box_violin.png")?;
    Ok(())
}
