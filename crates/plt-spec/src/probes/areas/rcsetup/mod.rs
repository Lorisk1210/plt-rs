//! Probe operations for the `rcsetup` area: validating rcParams from
//! rc-file text, as style sheets and `matplotlibrc` files set them.

mod canon;
mod fields;

use plt::rc::RcParams;

use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("rc.validate", validate)];

/// Sets `key` to the text `value` on matplotlib's defaults and returns
/// the parameter's canonical value.
fn validate(args: Args<'_>) -> Outcome {
    let key = args.str("key")?;
    let mut rc = RcParams::matplotlib();
    rc.set(key, args.str("value")?)
        .map_err(ProbeError::rejected)?;
    fields::canonical(&rc, key).ok_or_else(|| ProbeError::BadArgs(format!("{key} is not probed")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Json;

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
