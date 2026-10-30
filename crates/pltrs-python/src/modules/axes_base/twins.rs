//! Twinned axes (matplotlib's `twinx`, `twiny` and the background patch
//! of the bottom-most twin).

use pltrs::axes::Axes;

use super::axis_name;
use crate::binding::prelude::*;

/// A twin of `axes` sharing axis `name`, the other axis on the far side,
/// drawn `delta_zorder` above it (`twinx` for `"x"`, `twiny` for `"y"`).
fn twin(axes: Axes, name: String, delta_zorder: f64) -> PyResult<Axes> {
    Ok(axes.twin_with(axis_name(&name)?, delta_zorder)?)
}

/// Makes `twin`, made by its class at the place of `axes` and sharing
/// axis `name` with it, the twin of `axes` drawn `delta_zorder` above it
/// (`twinx(axes_class=...)`).
fn join_twin(axes: Axes, twin: Axes, name: String, delta_zorder: f64) -> PyResult<()> {
    axes.adopt_twin(&twin, axis_name(&name)?, delta_zorder)?;
    Ok(())
}

/// Shows the background patch of the bottom-most of the twins of `axes`
/// only (after `set_zorder`).
fn update_twinned_patch_visibility(axes: Axes) -> PyResult<()> {
    axes.update_twinned_patch_visibility();
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("twin", twin)?;
    module.add("join_twin", join_twin)?;
    module.add(
        "update_twinned_patch_visibility",
        update_twinned_patch_visibility,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;

    #[test]
    fn twins_below_show_the_background() {
        let ax = Figure::new().subplot();
        let below = twin(ax.clone(), "x".into(), -1.0).expect("a twin");
        assert!(below.patch_visible() && !ax.patch_visible());
        ax.set_zorder(-5.0);
        update_twinned_patch_visibility(ax.clone()).expect("updated");
        assert!(ax.patch_visible() && !below.patch_visible());
        assert!(twin(ax, "z".into(), 0.0).is_err());
    }
}
