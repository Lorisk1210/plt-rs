//! The box around a completed polygon, which moves and scales the
//! polygon with it (matplotlib's `PolygonSelector(draw_bounding_box=True)`).

use std::rc::Weak;

use pltrs_core::{Affine2D, Point};

use super::Polygon;
use crate::artist::ArtistHandle;
use crate::events::EventKind;
use crate::widgets::Connections;
use crate::widgets::selectors::driver::Core;
use crate::widgets::selectors::rectangle::{RectangleSelector, RectangleSelectorOptions};
use crate::widgets::selectors::{ModifierKeys, SelectorState};

/// The box around a completed polygon and what it last spanned.
#[derive(Debug)]
pub(super) struct BoundingBox {
    pub(super) selector: RectangleSelector,
    old_extents: (f64, f64, f64, f64),
    /// Rescales the polygon when the box changes.
    _scaling: Connections,
}

/// The bounds `(x0, y0, x1, y1)` of the finite `points` (matplotlib's
/// `Line2D.get_bbox`).
fn bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    points
        .iter()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), &(x, y)| (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        )
}

impl Polygon {
    /// Fits the box to the polygon (matplotlib's `_update_box`).
    pub(super) fn update_box(&mut self) {
        let (x0, y0, x1, y1) = bounds(&self.xys);
        if let Some(bounding) = &mut self.bounding {
            bounding.selector.set_extents((x0, x1, y0, y1));
            bounding.old_extents = bounding.selector.extents();
        }
    }

    /// Adds the box of a completed polygon (matplotlib's `_add_box`).
    pub(super) fn add_box(&mut self, core: &Core) {
        let defaults = RectangleSelectorOptions::default();
        let options = RectangleSelectorOptions {
            grab_range: self.grab_range,
            handle_props: self.box_handle_props.clone(),
            props: self.box_props.clone().unwrap_or(defaults.props),
            interactive: true,
            modifier_keys: ModifierKeys::default().without(SelectorState::Rotate),
            ..RectangleSelectorOptions::default()
        };
        let selector = RectangleSelector::new(&core.ax, options);
        let mut scaling = Connections::new(core.ax.figure());
        let this = Weak::clone(&self.this);
        scaling.connect(EventKind::Motion, move |_| {
            if let Some(shared) = this.upgrade() {
                let mut shared = shared.borrow_mut();
                let shared = &mut *shared;
                shared.tool.scale_to_box(&shared.core);
            }
        });
        let old_extents = selector.extents();
        self.bounding = Some(BoundingBox {
            selector,
            old_extents,
            _scaling: scaling,
        });
        self.update_box();
        if let Some(bounding) = &self.bounding {
            bounding.selector.edit_only();
        }
        self.draw(core);
    }

    /// Hides and drops the box (matplotlib's `_remove_box`).
    pub(super) fn remove_box(&mut self) {
        if let Some(bounding) = self.bounding.take() {
            bounding.selector.set_visible(false);
        }
    }

    /// Maps the polygon into the box after the box moved or was resized
    /// (matplotlib's `_scale_polygon`).
    pub(super) fn scale_to_box(&mut self, core: &Core) {
        if !core.selection_completed {
            return;
        }
        let Some(bounding) = &self.bounding else {
            return;
        };
        if bounding.old_extents == bounding.selector.extents() {
            return;
        }
        let (x1, y1, w1, h1) = bounding.selector.rect_bbox();
        let (x0, y0, xe, ye) = bounds(&self.xys);
        let transform = Affine2D::translation(-x0, -y0)
            .scale(1.0 / (xe - x0), 1.0 / (ye - y0))
            .scale(w1, h1)
            .translate(x1, y1);
        let mut verts: Vec<(f64, f64)> = self
            .verts()
            .iter()
            .map(|&(x, y)| {
                let p = transform.transform_point(Point::new(x, y));
                (p.x, p.y)
            })
            .collect();
        if let Some(&first) = verts.first() {
            verts.push(first);
        }
        self.xys = verts;
        self.draw(core);
        if let Some(bounding) = &mut self.bounding {
            bounding.old_extents = bounding.selector.extents();
        }
    }
}
