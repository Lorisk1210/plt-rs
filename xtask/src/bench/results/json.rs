//! The raw data written as JSON: every value of [`Results`], every run's
//! time in seconds and peak memory in KiB, indented so that a reader can
//! find their way.

use std::fmt::Write as _;

use pltrs_core::json::Json;

use super::super::cold::ColdResult;
use super::super::guard::{Guarded, Verdict};
use super::super::implementation::{Implementation, PerImplementation};
use super::super::machine::Machine;
use super::super::stats::Sampling;
use super::{Column, Plan, Results, WarmResult, WorkloadResult};

/// What the raw data's `format` member says, so that a reader can tell
/// the layout.
pub(super) const FORMAT: &str = "pltrs-bench-results 1";

/// `results` as indented JSON text.
pub(in super::super) fn to_text(results: &Results) -> String {
    let mut text = String::new();
    pretty(&mut text, &results_json(results), 0);
    text.push('\n');
    text
}

/// `results` as a JSON value.
fn results_json(results: &Results) -> Json {
    object([
        ("format", FORMAT.into()),
        ("machine", machine_json(&results.machine)),
        (
            "versions",
            object([
                ("matplotlib", pairs(&results.versions.reference)),
                ("pltrs-python", pairs(&results.versions.pltrs)),
            ]),
        ),
        (
            "load",
            object([
                ("before", results.load_before.as_str().into()),
                ("after", results.load_after.as_str().into()),
            ]),
        ),
        ("plan", plan_json(&results.plan)),
        (
            "startup",
            Json::Object(
                results
                    .startup
                    .iter()
                    .map(|s| (s.key.clone(), cold_json(&s.result)))
                    .collect(),
            ),
        ),
        (
            "workloads",
            Json::Array(results.workloads.iter().map(workload_json).collect()),
        ),
    ])
}

/// An object of `members`.
fn object<const N: usize>(members: [(&str, Json); N]) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// `(name, value)` pairs as an object of strings.
fn pairs(pairs: &[(String, String)]) -> Json {
    Json::Object(
        pairs
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().into()))
            .collect(),
    )
}

/// An unsigned count as a JSON integer.
fn count(value: u64) -> Json {
    Json::Integer(i64::try_from(value).unwrap_or(i64::MAX))
}

/// The machine.
fn machine_json(machine: &Machine) -> Json {
    object([
        ("cpu", machine.cpu.as_str().into()),
        ("cores", count(machine.cores as u64)),
        ("threads", count(machine.threads as u64)),
        ("memory_gib", machine.memory_gib.into()),
        ("os", machine.os.as_str().into()),
        ("kernel", machine.kernel.as_str().into()),
        ("affinity", machine.affinity.as_str().into()),
        ("governor", machine.governor.as_deref().into()),
        ("rustc", machine.rustc.as_str().into()),
        ("revision", machine.revision.as_str().into()),
    ])
}

/// How often things ran.
fn plan_json(plan: &Plan) -> Json {
    let sampling = |s: &Sampling| {
        object([
            ("min_runs", count(s.min_runs as u64)),
            ("min_seconds", s.min_seconds.into()),
            ("max_runs", count(s.max_runs as u64)),
        ])
    };
    object([
        ("warm", sampling(&plan.warm)),
        ("cold", sampling(&plan.cold)),
        ("startup", sampling(&plan.startup)),
    ])
}

/// One workload.
fn workload_json(w: &WorkloadResult) -> Json {
    let per = |values: &PerImplementation<Json>| {
        Json::Object(
            Implementation::ALL
                .iter()
                .map(|i| (i.key().to_owned(), values.get(*i).clone()))
                .collect(),
        )
    };
    let warm = PerImplementation::from_fn(|i| column_json(w.warm.get(i), warm_json));
    let cold = PerImplementation::from_fn(|i| column_json(w.cold.get(i), cold_json));
    object([
        ("name", w.name.as_str().into()),
        ("tier", w.tier.key().into()),
        ("format", w.format.as_str().into()),
        (
            "calls",
            w.calls.map_or(Json::Null, |calls| count(u64::from(calls))),
        ),
        (
            "guard",
            object([
                (
                    Implementation::Python.key(),
                    column_json(&w.guard.python, guarded_json),
                ),
                (
                    Implementation::Rust.key(),
                    column_json(&w.guard.rust, guarded_json),
                ),
            ]),
        ),
        ("warm", per(&warm)),
        ("cold", per(&cold)),
    ])
}

/// A column: its value, or why it has none.
fn column_json<T>(column: &Column<T>, value: impl Fn(&T) -> Json) -> Json {
    match column {
        Column::Measured(measured) => value(measured),
        Column::NotSupported(why) => object([("not_supported", why.as_str().into())]),
        Column::NotComparable(why) => object([("not_comparable", why.as_str().into())]),
    }
}

/// Warm runs.
fn warm_json(warm: &WarmResult) -> Json {
    object([
        ("seconds", warm.seconds.runs().to_vec().into()),
        ("bytes", count(warm.bytes)),
    ])
}

/// Cold runs.
fn cold_json(cold: &ColdResult) -> Json {
    object([
        ("seconds", cold.seconds.runs().to_vec().into()),
        (
            "peak_kib",
            Json::Array(cold.peaks_kib.iter().map(|&kib| count(kib)).collect()),
        ),
    ])
}

/// The guard's verdicts on a rendering.
fn guarded_json(guarded: &Guarded) -> Json {
    object([
        ("pixels", verdict_json(&guarded.pixels)),
        (
            "structure",
            guarded.structure.as_ref().map_or(Json::Null, verdict_json),
        ),
    ])
}

/// A verdict.
fn verdict_json(verdict: &Verdict) -> Json {
    match verdict {
        Verdict::Same(measure) => object([("same", (*measure).into())]),
        Verdict::Different(why) => object([("different", why.as_str().into())]),
        Verdict::Incomparable(why) => object([("incomparable", why.as_str().into())]),
    }
}

/// Writes `value` to `out` at `depth`: objects and arrays of containers
/// one member per line, arrays of plain values on one line.
fn pretty(out: &mut String, value: &Json, depth: usize) {
    let indent = |out: &mut String, depth: usize| {
        for _ in 0..depth {
            out.push_str("  ");
        }
    };
    match value {
        Json::Object(members) if !members.is_empty() => {
            out.push_str("{\n");
            for (i, (key, member)) in members.iter().enumerate() {
                indent(out, depth + 1);
                let _ = write!(out, "{}: ", pltrs_core::json::string(key));
                pretty(out, member, depth + 1);
                out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push('}');
        }
        Json::Array(items)
            if items
                .iter()
                .any(|item| matches!(item, Json::Object(_) | Json::Array(_))) =>
        {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                indent(out, depth + 1);
                pretty(out, item, depth + 1);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push(']');
        }
        plain => {
            let _ = write!(out, "{plain}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::from_text;
    use super::super::tests::sample;
    use super::*;

    #[test]
    fn the_raw_data_reads_back_as_written() {
        let results = sample();
        let text = to_text(&results);
        assert!(text.starts_with("{\n  \"format\": \"pltrs-bench-results 1\",\n"));
        assert_eq!(from_text(&text).expect("valid raw data"), results);
    }

    #[test]
    fn plain_arrays_stay_on_one_line() {
        let mut out = String::new();
        let value = object([("a", vec![1.0, 2.5].into()), ("b", Json::Array(Vec::new()))]);
        pretty(&mut out, &value, 0);
        assert_eq!(out, "{\n  \"a\": [1.0, 2.5],\n  \"b\": []\n}");
    }
}
