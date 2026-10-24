//! The subplot configuration tool (matplotlib's `widgets.SubplotTool`):
//! sliders for the margins and spacing of another figure's subplots.

use std::cell::RefCell;
use std::rc::Rc;

use super::{Button, ButtonOptions, Slider, SliderOptions};
use crate::artist::ArtistHandle;
use crate::figure::Figure;
use crate::gridspec::{SubplotOverrides, SubplotParams};

/// The parameters in slider order, as matplotlib labels them.
const NAMES: [&str; 6] = ["left", "bottom", "right", "top", "wspace", "hspace"];

/// `params` in slider order.
fn values(params: &SubplotParams) -> [f64; 6] {
    [
        params.left,
        params.bottom,
        params.right,
        params.top,
        params.wspace,
        params.hspace,
    ]
}

/// Applies the slider values to `target`.
fn apply(target: &Figure, [left, bottom, right, top, wspace, hspace]: [f64; 6]) {
    let overrides = SubplotOverrides {
        left: Some(left),
        bottom: Some(bottom),
        right: Some(right),
        top: Some(top),
        wspace: Some(wspace),
        hspace: Some(hspace),
    };
    // An invalid combination (left at or beyond right, ...), which the
    // slider links normally prevent, keeps the previous parameters.
    let _ = target.subplots_adjust(&overrides);
}

/// Six sliders in one figure adjusting the subplot parameters of another,
/// with a button resetting them.
#[derive(Debug)]
pub struct SubplotTool {
    sliders: Rc<Vec<Slider>>,
    reset: Button,
}

impl SubplotTool {
    /// Fills `tool` with sliders for the subplot parameters of `target`.
    pub fn new(target: &Figure, tool: &Figure) -> Self {
        let layout = SubplotOverrides {
            left: Some(0.2),
            right: Some(0.9),
            ..SubplotOverrides::default()
        };
        tool.subplots_adjust(&layout)
            .expect("0.2 and 0.9 are valid margins");
        tool.suptitle("Click on slider to adjust subplot param");
        let grid = tool
            .subplots(NAMES.len() + 1, 1)
            .expect("a 7×1 grid has cells");
        let axes = grid.as_slice();
        let current = Rc::new(RefCell::new(values(&target.subplot_params())));
        let sliders: Vec<Slider> = NAMES
            .iter()
            .zip(axes)
            .enumerate()
            .map(|(index, (name, ax))| {
                ax.set_navigate(false);
                let options = SliderOptions {
                    valinit: current.borrow()[index],
                    ..SliderOptions::default()
                };
                let slider = Slider::new(ax, name, 0.0, 1.0, options)
                    .expect("the unit interval is a finite range");
                let (target, current) = (target.clone(), Rc::clone(&current));
                slider.on_changed(move |value| {
                    let mut values = current.borrow_mut();
                    values[index] = value;
                    apply(&target, *values);
                });
                slider
            })
            .collect();
        if let Some(spare) = axes.last() {
            spare.remove();
        }
        let [left, bottom, right, top, wspace, hspace] = &sliders[..] else {
            unreachable!("one slider per parameter");
        };
        for slider in [left, bottom, wspace, hspace] {
            slider.set_closed(true, false);
        }
        for slider in [right, top] {
            slider.set_closed(false, true);
        }
        left.set_slidermax(right);
        right.set_slidermin(left);
        bottom.set_slidermax(top);
        top.set_slidermin(bottom);
        let sliders = Rc::new(sliders);
        let reset = Button::new(
            &tool
                .add_axes([0.8, 0.05, 0.15, 0.075])
                .expect("the reset button's rectangle is valid"),
            "Reset",
            ButtonOptions::default(),
        );
        {
            let (sliders, target) = (Rc::clone(&sliders), target.clone());
            reset.on_clicked(move || {
                for slider in sliders.iter() {
                    slider.reset_quietly();
                }
                let values = std::array::from_fn(|i| sliders[i].val());
                *current.borrow_mut() = values;
                apply(&target, values);
            });
        }
        Self { sliders, reset }
    }

    /// The sliders for left, bottom, right, top, wspace and hspace.
    pub fn sliders(&self) -> &[Slider] {
        &self.sliders
    }

    /// The reset button.
    pub fn reset_button(&self) -> &Button {
        &self.reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliders_adjust_the_target_and_reset() {
        let target = Figure::new();
        target.subplots(2, 2).unwrap();
        let tool = Figure::with_size(6.0, 3.0, 100.0);
        let subplot_tool = SubplotTool::new(&target, &tool);
        assert_eq!(subplot_tool.sliders().len(), 6);
        assert_eq!(tool.axes().len(), 7, "six sliders and the reset button");
        subplot_tool.sliders()[0].set_val(0.3);
        assert_eq!(target.subplot_params().left, 0.3);
        // Left stops at right.
        let right = subplot_tool.sliders()[2].val();
        subplot_tool.sliders()[0].set_val(0.95);
        assert!(target.subplot_params().left <= right);
    }
}
