//! The `LassoSelector` and `Lasso` ops.

use std::cell::RefCell;
use std::rc::Rc;

use plt::axes::Axes;
use plt::lines::LineProps;
use plt::widgets::{Lasso, LassoSelector};

use super::region::alpha;
use super::steps::{Attempt, Tool, replay, unknown};
use super::{bad, button, get_ax, props, rgba};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

impl Tool for LassoSelector {
    fn ax(&self) -> Axes {
        LassoSelector::ax(self)
    }

    fn call(&self, method: &str, args: &[Json]) -> Result<Attempt, ProbeError> {
        match method {
            "set_props" => self.set_props(props::line(args.first().ok_or_else(|| bad(method))?)?),
            other => return Err(unknown("method", other)),
        }
        Ok(Attempt::Done)
    }

    fn set(&self, attribute: &str, _value: &Json) -> Result<Attempt, ProbeError> {
        Err(unknown("attribute", attribute))
    }

    fn get(&self, query: &str) -> Result<Json, ProbeError> {
        let line = self.artist();
        match query {
            "_selection_artist.get_color" => rgba(&line.color()),
            "_selection_artist.get_alpha" => Ok(alpha(&line)),
            other => Err(unknown("query", other)),
        }
    }
}

/// `LassoSelector(ax, onselect, **options)` driven by `steps`.
pub(super) fn lasso_selector(args: Args<'_>) -> Outcome {
    styled(args, || {
        let mut line_props = LineProps::default();
        let mut buttons = None;
        for (name, value) in args
            .json("options")?
            .as_object()
            .ok_or_else(|| bad("options"))?
        {
            match name.as_str() {
                // Blitting only speeds drawing up; selections are the same.
                "useblit" => {}
                "props" => line_props = props::line(value)?,
                "button" => buttons = Some(vec![button(value)?]),
                other => return Err(unknown("option", other)),
            }
        }
        let tool = LassoSelector::new(&get_ax()?, line_props, buttons);
        let selects: Rc<RefCell<Vec<Json>>> = Rc::default();
        let sink = Rc::clone(&selects);
        tool.on_select(move |verts| {
            let verts = verts
                .iter()
                .map(|&(x, y)| Json::Array(vec![Json::from(x), Json::from(y)]))
                .collect();
            sink.borrow_mut().push(Json::Array(verts));
        });
        let steps = replay(&tool, args.json("steps")?)?;
        let selects = Json::Array(selects.borrow().clone());
        Ok(encode::object([("steps", steps), ("selects", selects)]))
    })
}

/// The line of `Lasso(ax, (100, 100), onselect, props=props)`, recoloured
/// as asked: `{color, linestyle, lw, alpha}`.
pub(super) fn lasso(args: Args<'_>) -> Outcome {
    styled(args, || {
        let props = args.get("props").map(props::line).transpose()?;
        let tool = Lasso::new(&get_ax()?, (100.0, 100.0), props.unwrap_or_default());
        let line = tool.line();
        if let Some(color) = args.opt_str("set_color")? {
            line.set_color(color);
        }
        if let Some(value) = args.opt_f64("set_alpha")? {
            plt::artist::ArtistHandle::set_alpha(&line, Some(value));
        }
        Ok(encode::object([
            ("color", rgba(&line.color())?),
            ("linestyle", line.linestyle().canon()),
            ("lw", Json::from(line.linewidth())),
            ("alpha", alpha(&line)),
        ]))
    })
}
