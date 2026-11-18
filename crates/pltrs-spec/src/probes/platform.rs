//! Platform goldens of the probes.
//!
//! `spec/goldens/probes/<area>.json` is what matplotlib computes on Linux
//! x86-64 with AVX-512. Where it computes something else on the platform
//! this harness is built for, or at the processor level it runs at,
//! `spec/goldens/platforms/<target, level or target+level>/probes/<area>.json`
//! (written by the reference runner on that platform or at that level,
//! see `spec/reference/spec_support/golden.py`) holds the probes that
//! differ:
//! those with another outcome, those with other arguments (matplotlib and
//! numpy compute many of them), and the keys of the common probes
//! matplotlib does not evaluate there.

use std::collections::BTreeMap;

use super::{LoadError, Probe, ProbeFile, parse_probe};
use crate::json::Json;

/// What names a probe within its area: the operation and its arguments,
/// with object members in key order (Python's `sort_keys`).
fn key(op: &str, args: &Json) -> String {
    format!("{op} {}", sorted(args))
}

/// `value` with the members of every object in key order.
fn sorted(value: &Json) -> Json {
    match value {
        Json::Array(items) => Json::Array(items.iter().map(sorted).collect()),
        Json::Object(members) => {
            let mut members: Vec<(String, Json)> = members
                .iter()
                .map(|(name, member)| (name.clone(), sorted(member)))
                .collect();
            members.sort_by(|a, b| a.0.cmp(&b.0));
            Json::Object(members)
        }
        other => other.clone(),
    }
}

impl ProbeFile {
    /// The probes this platform is held to: `self`, the common golden,
    /// with the probes of the platform golden `platform` in place of those
    /// they replace, without those it lists as absent, and with those it
    /// adds, which go last.
    pub fn with_platform(self, platform: &Json) -> Result<Self, LoadError> {
        let replacements = platform
            .get("probes")
            .and_then(Json::as_array)
            .ok_or(LoadError::Layout("missing \"probes\" array"))?
            .iter()
            .map(parse_probe)
            .collect::<Result<Vec<Probe>, _>>()?;
        let absent = platform
            .get("absent")
            .and_then(Json::as_array)
            .ok_or(LoadError::Layout("missing \"absent\" array"))?;
        let mut changes = BTreeMap::new();
        for entry in absent {
            let op = entry
                .get("op")
                .and_then(Json::as_str)
                .ok_or(LoadError::Layout("absent probe without \"op\""))?;
            let args = entry
                .get("args")
                .ok_or(LoadError::Layout("absent probe without \"args\""))?;
            changes.insert(key(op, args), None);
        }
        let mut added = Vec::new();
        for probe in replacements {
            added.push(key(&probe.op, &probe.args));
            changes.insert(key(&probe.op, &probe.args), Some(probe));
        }
        let mut probes = Vec::with_capacity(self.probes.len() + added.len());
        for probe in self.probes {
            match changes.get_mut(&key(&probe.op, &probe.args)) {
                None => probes.push(probe),
                Some(replacement) => probes.extend(replacement.take()),
            }
        }
        // What replaced nothing is new on this platform.
        probes.extend(added.iter().filter_map(|k| changes.get_mut(k)?.take()));
        Ok(Self {
            area: self.area,
            probes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probes::Expected;

    fn common() -> ProbeFile {
        let doc = Json::parse(
            r#"{"area": "demo", "probes": [
                {"op": "f", "args": {"x": 1, "y": [NaN]}, "value": 1.0},
                {"op": "f", "args": {"x": 2}, "value": 2.0},
                {"op": "g", "args": {"x": 0.30000000000000004}, "value": 3.0}
            ]}"#,
        )
        .unwrap();
        ProbeFile::parse(&doc).unwrap()
    }

    #[test]
    fn platform_probes_replace_remove_and_add() {
        let platform = Json::parse(
            r#"{"area": "demo", "platform": "p", "probes": [
                {"op": "f", "args": {"y": [NaN], "x": 1}, "value": 1.5},
                {"op": "g", "args": {"x": 0.3}, "error": "ValueError"}
            ], "absent": [{"op": "g", "args": {"x": 0.30000000000000004}}]}"#,
        )
        .unwrap();
        let file = common().with_platform(&platform).unwrap();
        let outcomes: Vec<(&str, &Expected)> = file
            .probes
            .iter()
            .map(|p| (p.op.as_str(), &p.expected))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("f", &Expected::Value(Json::from(1.5))),
                ("f", &Expected::Value(Json::from(2.0))),
                ("g", &Expected::Error("ValueError".to_owned())),
            ]
        );
        assert_eq!(file.probes[2].args.get("x"), Some(&Json::from(0.3)));
    }

    #[test]
    fn malformed_platform_goldens_are_refused() {
        let no_absent = Json::parse(r#"{"probes": []}"#).unwrap();
        assert!(common().with_platform(&no_absent).is_err());
        let unnamed = Json::parse(r#"{"probes": [], "absent": [{"args": {}}]}"#).unwrap();
        assert!(common().with_platform(&unnamed).is_err());
    }
}
