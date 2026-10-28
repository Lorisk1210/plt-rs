//! The check button, radio button and text box ops.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use plt::artist::ArtistHandle;
use plt::colors::Color;
use plt::rc;
use plt::transforms::Point;
use plt::widgets::{
    ButtonMarkerProps, CheckButtons, RadioButtons, RadioButtonsOptions, TextBox, TextBoxOptions,
};

use super::steps::{key_press, press};
use super::{count, get_ax, malformed};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The probe's `labels`.
fn labels(args: Args<'_>) -> Result<Vec<String>, ProbeError> {
    args.json("labels")?
        .as_array()
        .ok_or_else(|| malformed("labels"))?
        .iter()
        .map(|label| {
            label
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| malformed("labels"))
        })
        .collect()
}

/// The probe's `actives`.
fn actives(args: Args<'_>) -> Result<Vec<bool>, ProbeError> {
    args.json("actives")?
        .as_array()
        .ok_or_else(|| malformed("actives"))?
        .iter()
        .map(|active| active.as_bool().ok_or_else(|| malformed("actives")))
        .collect()
}

fn flags(values: &[bool]) -> Json {
    Json::Array(values.iter().map(|&v| Json::from(v)).collect())
}

fn texts(values: &[String]) -> Json {
    Json::Array(values.iter().map(|v| Json::from(v.as_str())).collect())
}

/// Check buttons on `get_ax` axes made from the probe's labels and actives.
fn check(args: Args<'_>) -> Result<CheckButtons, ProbeError> {
    let labels = labels(args)?;
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    Ok(CheckButtons::new(&get_ax()?, &labels, &actives(args)?))
}

/// `test_CheckButtons`: the status through `set_active(0)` and `clear()`.
pub(super) fn check_buttons(args: Args<'_>) -> Outcome {
    styled(args, || {
        let check = check(args)?;
        let status = check.status();
        check
            .set_active(0, None)
            .map_err(|_| malformed("no first button"))?;
        let (after, checked) = (check.status(), check.checked_labels());
        check.clear();
        let cid = check.on_clicked(|_| {});
        Ok(encode::object([
            ("status", flags(&status)),
            ("after_set_active", flags(&after)),
            ("checked", texts(&checked)),
            ("after_clear", flags(&check.status())),
            ("checked_after_clear", texts(&check.checked_labels())),
            ("disconnected", Json::from(check.disconnect(cid))),
        ]))
    })
}

/// `CheckButtons.set_active(index)`: the status after it, or the error
/// for an index out of range.
pub(super) fn check_set_active(args: Args<'_>) -> Outcome {
    styled(args, || {
        let check = check(args)?;
        let index = usize::try_from(args.i64("index")?)
            .map_err(|_| ProbeError::rejected("a negative index"))?;
        check
            .set_active(index, None)
            .map_err(ProbeError::rejected)?;
        Ok(flags(&check.status()))
    })
}

/// `test_RadioButtons`: the selection after `set_active(1)` and after
/// `clear()`.
pub(super) fn radio_buttons(args: Args<'_>) -> Outcome {
    styled(args, || {
        let labels = labels(args)?;
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let radio = RadioButtons::new(&get_ax()?, &labels, 0);
        let selection = |radio: &RadioButtons| {
            Json::Array(vec![
                Json::from(radio.value_selected()),
                count(radio.index_selected()),
            ])
        };
        radio
            .set_active(1)
            .map_err(|_| malformed("no second button"))?;
        let selected = selection(&radio);
        radio.clear();
        Ok(Json::Array(vec![selected, selection(&radio)]))
    })
}

/// The face colours of radio buttons made with both an active colour and
/// a face colour in their marker properties.
pub(super) fn radio_activecolor(args: Args<'_>) -> Outcome {
    styled(args, || {
        let labels = labels(args)?;
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let options = RadioButtonsOptions {
            activecolor: Some(Color::from(args.str("activecolor")?)),
            radio_props: ButtonMarkerProps {
                facecolors: Some(vec![Color::from(args.str("facecolor")?)]),
                ..ButtonMarkerProps::default()
            },
            ..RadioButtonsOptions::default()
        };
        let radio = RadioButtons::with_options(&get_ax()?, &labels, 0, options)
            .map_err(ProbeError::rejected)?;
        let colors = radio.buttons().colors().map_err(ProbeError::rejected)?;
        Ok(Json::Array(
            colors.faces.into_iter().map(encode::rgba).collect(),
        ))
    })
}

/// How often the callbacks of one-button radio or check buttons run after
/// a press at the button, axes point `(0.15, 0.5)`.
pub(super) fn on_clicked(args: Args<'_>) -> Outcome {
    styled(args, || {
        let ax = get_ax()?;
        let clicks = Rc::new(Cell::new(0_usize));
        let counter = Rc::clone(&clicks);
        let label = ["Test Button"];
        // Keep the widget alive while the press is processed.
        let _widget: Box<dyn std::any::Any> = match args.str("widget")? {
            "RadioButtons" => {
                let radio = RadioButtons::new(&ax, &label, 0);
                radio.on_clicked(move |_| counter.set(counter.get() + 1));
                Box::new(radio)
            }
            "CheckButtons" => {
                let check = CheckButtons::new(&ax, &label, &[false]);
                check.on_clicked(move |_| counter.set(counter.get() + 1));
                Box::new(check)
            }
            other => return Err(ProbeError::BadArgs(format!("unknown widget {other:?}"))),
        };
        let figure = ax.figure();
        let resolve = |transform| {
            figure
                .resolve_transform(&transform)
                .map_err(ProbeError::rejected)
        };
        let display = resolve(ax.trans_axes())?.transform_point(Point::new(0.15, 0.5));
        let data = resolve(ax.trans_data())?
            .inverted()
            .ok_or_else(|| ProbeError::rejected("singular data transform"))?
            .transform_point(display);
        press(&ax, (data.x, data.y))?;
        Ok(count(clicks.get()))
    })
}

/// `test_TextBox` with rcParams `toolbar` set: the text and the callback
/// counts through `set_val`, `begin_typing`, `stop_typing`, a click and
/// two keys.
pub(super) fn textbox(args: Args<'_>) -> Outcome {
    styled(args, || {
        rc::set("toolbar", args.str("toolbar")?).map_err(ProbeError::rejected)?;
        let ax = get_ax()?;
        let tool = TextBox::new(&ax, "", "", TextBoxOptions::default());
        let changes = Rc::new(RefCell::new(0_usize));
        let submits = Rc::new(RefCell::new(0_usize));
        let (change_sink, submit_sink) = (Rc::clone(&changes), Rc::clone(&submits));
        tool.on_submit(move |_| *submit_sink.borrow_mut() += 1);
        tool.on_text_change(move |_| *change_sink.borrow_mut() += 1);
        let text = tool.text();
        press(&ax, (0.5, 0.5))?;
        tool.set_val("x**2");
        let (after, changed) = (tool.text(), *changes.borrow());
        tool.begin_typing();
        tool.stop_typing();
        let submitted = *submits.borrow();
        press(&ax, (0.5, 0.5))?;
        key_press(&ax, "+")?;
        key_press(&ax, "5")?;
        let changed_again = *changes.borrow();
        Ok(Json::Array(vec![
            Json::from(text),
            Json::from(after),
            count(changed),
            count(submitted),
            count(changed_again),
        ]))
    })
}
