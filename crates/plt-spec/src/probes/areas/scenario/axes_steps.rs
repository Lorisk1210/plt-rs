//! Steps acting on an axes, dispatched by topic, and `Axes.set(**kwargs)`.

use plt::axes::Axes;

use super::call::Call;
use super::scene::{Handled, Produced, Scene};
use super::{
    axes_fills, axes_images, axes_labels, axes_layout, axes_limits, axes_plot, axes_ticks,
    legend_steps,
};
use crate::probes::ProbeError;

/// Runs a method of `ax`.
pub(super) fn call(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Handled {
    if call.method == "set" {
        return Some(set(scene, ax, call));
    }
    axes_limits::call(ax, call)
        .or_else(|| axes_layout::call(scene, ax, call))
        .or_else(|| axes_labels::call(ax, call))
        .or_else(|| axes_ticks::call(ax, call))
        .or_else(|| axes_plot::call(scene, ax, call))
        .or_else(|| axes_images::call(ax, call))
        .or_else(|| axes_fills::call(ax, call))
        .or_else(|| legend_steps::axes_call(scene, ax, call))
}

/// matplotlib's `Artist.set(**kwargs)`: `set_<name>(value)` for each
/// keyword, in order.
fn set(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    for (name, value) in call.keywords() {
        call.keyword(name);
        let method = format!("set_{name}");
        let args = [value.clone()];
        let setter = Call::new(&method, &args)?;
        self::call(scene, ax, &setter)
            .unwrap_or_else(|| Err(setter.bad("unsupported property")))?;
        setter.finish()?;
    }
    Ok(Produced::Nothing)
}
