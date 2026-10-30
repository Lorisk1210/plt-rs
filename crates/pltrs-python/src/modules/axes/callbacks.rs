//! The limit callbacks of an axes (matplotlib's `ax.callbacks` signals
//! `xlim_changed` and `ylim_changed`): pltrs (Rust) tells the package when
//! the view limits of an axis change, and the package's
//! `CallbackRegistry` calls what was connected to it.

use pltrs::axes::{Axes, AxisName};

use crate::binding::prelude::*;

/// Calls `notify` without arguments whenever the view limits of the axis
/// `name` (`"x"` or `"y"`) of `axes` change, until the axes is cleared.
fn on_limits_changed(axes: Axes, name: String, notify: Callback) -> PyResult<()> {
    let axis = match name.as_str() {
        "x" => AxisName::X,
        "y" => AxisName::Y,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for name; supported values are 'x', 'y'"
            )));
        }
    };
    axes.on_limits_changed(axis, move |_| {
        // What the registry returns is of no use; an exception it raised
        // goes to the native call that changed the limits.
        let _ = notify.call::<Owned>(());
    });
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("on_limits_changed", on_limits_changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn limit_changes_reach_the_callable() {
        with_python(|py| {
            let ax = Figure::new().subplot();
            let calls = eval(py, "", "[]");
            let append = calls.getattr("append").expect("a list method");
            let notify = eval(py, "import functools", "functools.partial")
                .call(&[append, "x".into_py(py).expect("a string")], &[])
                .expect("a partial");
            on_limits_changed(ax.clone(), "x".into(), notify.extract().expect("callable"))
                .expect("a valid axis");
            ax.set_xlim(0.0, 2.0).expect("finite limits");
            ax.set_ylim(0.0, 2.0).expect("finite limits");
            assert_eq!(calls.extract::<Vec<String>>().expect("strings"), ["x"]);
            assert!(
                on_limits_changed(ax, "z".into(), notify.extract().expect("callable")).is_err()
            );
        });
    }
}
