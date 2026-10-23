//! The last two figures of matplotlib's legend guide: handlers of the
//! user's own, one for an object that is no artist and one drawing a
//! circle as an ellipse.

use plt::legend::{
    HandlerArtist, HandlerEntry, HandlerKey, LegendError, LegendHandle, LegendHandler,
    LegendOptions,
};
use plt::patches::{Circle, Ellipse, Rectangle};
use plt::prelude::*;

use crate::figures::CaseResult;

/// `legend_guide`'s `AnyObject`, a handle that is no artist.
#[derive(Debug)]
struct AnyObject;

/// `legend_guide`'s `AnyObjectHandler`: a red rectangle hatched with
/// `xx`, edged black 3 points wide, filling the handle box.
#[derive(Debug)]
struct AnyObjectHandler;

impl LegendHandler for AnyObjectHandler {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let area = entry.area();
        let patch = Rectangle::new((area.xdescent, area.ydescent), area.width, area.height);
        let props = PatchProps::default()
            .facecolor("red")
            .edgecolor("black")
            .hatch("xx")
            .linewidth(3.0);
        Ok(vec![HandlerArtist::patch(patch, props)])
    }
}

pub(super) fn legend_guide_11(fig: &Figure) -> CaseResult {
    fig.subplot().legend(
        LegendOptions::default()
            .handles([LegendHandle::object(AnyObject)])
            .labels(["My first handler"])
            .handler(HandlerKey::of::<AnyObject>(), AnyObjectHandler),
    )?;
    Ok(())
}

/// `legend_guide`'s `HandlerEllipse`: an ellipse filling the handle box,
/// drawn like the original patch.
#[derive(Debug)]
struct HandlerEllipse;

impl LegendHandler for HandlerEllipse {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let area = entry.area();
        let center = (
            0.5 * area.width - 0.5 * area.xdescent,
            0.5 * area.height - 0.5 * area.ydescent,
        );
        let ellipse = Ellipse::new(
            center,
            area.width + area.xdescent,
            area.height + area.ydescent,
        );
        Ok(vec![entry.patch_like_original(ellipse)])
    }
}

pub(super) fn legend_guide_12(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let c = ax.add_patch_with(
        Circle::new((0.5, 0.5), 0.25),
        PatchProps::default()
            .facecolor("green")
            .edgecolor("red")
            .linewidth(3.0),
    );
    ax.legend(
        LegendOptions::default()
            .handles([&c])
            .labels(["An ellipse, not a rectangle"])
            .handler(HandlerKey::of::<Circle>(), HandlerEllipse),
    )?;
    Ok(())
}
