//! Probe operations for the `rcsetup` area: validating rcParams from
//! rc-file text, as style sheets and `matplotlibrc` files set them, and
//! the parameters each bundled style sheet sets.

mod fields;

use plt::rc::RcParams;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("rc.validate", validate),
    Op::new("style.sheet", style_sheet),
];

/// Sets `key` to the text `value` on matplotlib's defaults and returns
/// the parameter's canonical value.
fn validate(args: Args<'_>) -> Outcome {
    let key = args.str("key")?;
    let mut rc = RcParams::matplotlib();
    rc.set(key, args.str("value")?)
        .map_err(ProbeError::rejected)?;
    fields::canonical(&rc, key).ok_or_else(|| ProbeError::BadArgs(format!("{key} is not probed")))
}

/// The canonical value of every parameter the style sheet `name` sets,
/// keys sorted.
fn style_sheet(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    let sheet = plt::style::sheet(name)
        .ok_or_else(|| ProbeError::BadArgs(format!("no style sheet {name:?}")))?;
    let mut values = Vec::new();
    for entry in sheet.entries().map_err(ProbeError::rejected)? {
        let mut rc = RcParams::matplotlib();
        rc.set(&entry.key, &entry.value)
            .map_err(ProbeError::rejected)?;
        if let Some(value) = fields::canonical(&rc, &entry.key) {
            values.push((entry.key, value));
        }
    }
    values.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Json::Object(values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_come_back_canonical() {
        let mut rc = RcParams::matplotlib();
        rc.set("lines.linestyle", "--").unwrap();
        assert_eq!(
            fields::canonical(&rc, "lines.linestyle"),
            Some(Json::String("dashed".to_owned()))
        );
        assert_eq!(fields::canonical(&rc, "path.effects"), None);
    }
}
