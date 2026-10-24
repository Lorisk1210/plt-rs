//! Probe operations for the `animation` area (the JavaScript player of
//! `matplotlib.animation`).

use std::num::NonZeroU32;
use std::time::Duration;

use plt::animation::{AnimationOptions, Frames, FuncAnimation, HtmlOptions, PlayMode};
use plt::figure::Figure;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("animation.jshtml", jshtml)];

/// What the reference module writes for the base64 payload of a frame.
const MASKED_DATA: &str = "base64,DATA\"";

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

fn jshtml(args: Args<'_>) -> Outcome {
    let count = usize::try_from(args.i64("frames")?).map_err(|_| bad("frames"))?;
    let interval = u64::try_from(args.i64("interval")?).map_err(|_| bad("interval"))?;
    let options = AnimationOptions {
        interval: Duration::from_millis(interval),
        repeat: args.bool("repeat")?,
        ..AnimationOptions::default()
    };
    let fps = args
        .opt_i64("fps")?
        .map(|fps| {
            u32::try_from(fps)
                .ok()
                .and_then(NonZeroU32::new)
                .ok_or_else(|| bad("fps"))
        })
        .transpose()?;
    let default_mode = args
        .opt_str("default_mode")?
        .map(|mode| match mode {
            "once" => Ok(PlayMode::Once),
            "loop" => Ok(PlayMode::Loop),
            "reflect" => Ok(PlayMode::Reflect),
            _ => Err(bad("default_mode")),
        })
        .transpose()?;
    let fig = Figure::with_size(1.0, 1.0, 20.0);
    let animation = FuncAnimation::new(&fig, Frames::Count(count), options, |_| {});
    let html = animation
        .to_jshtml(HtmlOptions { fps, default_mode })
        .map_err(ProbeError::rejected)?;
    Ok(Json::from(mask_ids(&mask_data(&html))))
}

/// `html` with every frame's base64 payload replaced, as the reference
/// module's `re.sub(r'base64,[^"]*"', 'base64,DATA"', html)`.
fn mask_data(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("base64,") {
        let payload = &rest[start + "base64,".len()..];
        let Some(end) = payload.find('"') else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(MASKED_DATA);
        rest = &payload[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `html` with each run of 32 lowercase hex digits replaced by `ID`, as the
/// reference module's `re.sub(r"[0-9a-f]{32}", "ID", html)`.
fn mask_ids(html: &str) -> String {
    const ID_LEN: usize = 32;
    let is_digit = |byte: &u8| matches!(byte, b'0'..=b'9' | b'a'..=b'f');
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut copied = 0;
    let mut at = 0;
    while at + ID_LEN <= bytes.len() {
        if bytes[at..at + ID_LEN].iter().all(is_digit) {
            out.push_str(&html[copied..at]);
            out.push_str("ID");
            at += ID_LEN;
            copied = at;
        } else {
            at += 1;
        }
    }
    out.push_str(&html[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_and_ids_are_masked_like_the_reference() {
        let html =
            "x = \"data:image/png;base64,iVBO\\\nRw==\" anim0123456789abcdef0123456789abcdef.go";
        let masked = mask_ids(&mask_data(html));
        assert_eq!(masked, "x = \"data:image/png;base64,DATA\" animID.go");
        assert_eq!(mask_ids(&"a".repeat(65)), "IDIDa");
        assert_eq!(mask_data("base64,open"), "base64,open");
    }
}
