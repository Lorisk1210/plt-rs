//! The raw data read back, so that the report can be written again from
//! it (`cargo xtask bench --rewrite-report`) without measuring.

use pltrs_core::json::Json;

use super::super::cold::ColdResult;
use super::super::guard::{Guarded, Verdict};
use super::super::implementation::{Implementation, PerImplementation};
use super::super::machine::Machine;
use super::super::stats::{Samples, Sampling};
use super::super::workloads::Tier;
use super::json::FORMAT;
use super::{Column, Guard, Plan, Results, Startup, Versions, WarmResult, WorkloadResult};

/// A value of the raw data and where it is, for error messages.
#[derive(Clone)]
struct Value<'a> {
    /// The value.
    json: &'a Json,
    /// Its path from the root (`workloads[3].warm`).
    path: String,
}

impl<'a> Value<'a> {
    /// The root of the raw data.
    fn root(json: &'a Json) -> Self {
        Self {
            json,
            path: "the raw data".to_owned(),
        }
    }

    /// An error about this value.
    fn error(&self, what: &str) -> String {
        format!("results.json: {} {what}", self.path)
    }

    /// The member `key`.
    fn member(&self, key: &str) -> Result<Self, String> {
        let json = self
            .json
            .get(key)
            .ok_or_else(|| self.error(&format!("has no member {key:?}")))?;
        Ok(Self {
            json,
            path: format!("{}.{key}", self.path),
        })
    }

    /// The member `key`, unless it is missing or null.
    fn optional(&self, key: &str) -> Option<Self> {
        self.member(key).ok().filter(|value| !value.json.is_null())
    }

    /// The members, each with its key.
    fn members(&self) -> Result<Vec<(String, Self)>, String> {
        let members = self
            .json
            .as_object()
            .ok_or_else(|| self.error("is not an object"))?;
        Ok(members
            .iter()
            .map(|(key, json)| {
                let value = Self {
                    json,
                    path: format!("{}.{key}", self.path),
                };
                (key.clone(), value)
            })
            .collect())
    }

    /// The elements.
    fn elements(&self) -> Result<Vec<Self>, String> {
        let elements = self
            .json
            .as_array()
            .ok_or_else(|| self.error("is not an array"))?;
        Ok(elements
            .iter()
            .enumerate()
            .map(|(i, json)| Self {
                json,
                path: format!("{}[{i}]", self.path),
            })
            .collect())
    }

    /// The text.
    fn text(&self) -> Result<String, String> {
        self.json
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| self.error("is not a string"))
    }

    /// The number.
    fn number(&self) -> Result<f64, String> {
        self.json
            .as_f64()
            .ok_or_else(|| self.error("is not a number"))
    }

    /// The count.
    fn count<T: TryFrom<i64>>(&self) -> Result<T, String> {
        self.json
            .as_i64()
            .and_then(|n| T::try_from(n).ok())
            .ok_or_else(|| self.error("is not a count"))
    }

    /// The text of member `key`.
    fn text_of(&self, key: &str) -> Result<String, String> {
        self.member(key)?.text()
    }

    /// The count of member `key`.
    fn count_of<T: TryFrom<i64>>(&self, key: &str) -> Result<T, String> {
        self.member(key)?.count()
    }
}

/// The results written in `text` by `to_text`.
pub(in super::super) fn from_text(text: &str) -> Result<Results, String> {
    let json = Json::parse(text).map_err(|err| format!("results.json: {err}"))?;
    let root = Value::root(&json);
    if root.text_of("format")? != FORMAT {
        return Err(format!("results.json is not in the layout {FORMAT:?}"));
    }
    let load = root.member("load")?;
    let versions = root.member("versions")?;
    Ok(Results {
        machine: machine(&root.member("machine")?)?,
        versions: Versions {
            reference: pairs(&versions.member("matplotlib")?)?,
            pltrs: pairs(&versions.member("pltrs-python")?)?,
        },
        load_before: load.text_of("before")?,
        load_after: load.text_of("after")?,
        plan: plan(&root.member("plan")?)?,
        startup: root
            .member("startup")?
            .members()?
            .into_iter()
            .map(|(key, value)| {
                Ok(Startup {
                    key,
                    result: cold(&value)?,
                })
            })
            .collect::<Result<_, String>>()?,
        workloads: root
            .member("workloads")?
            .elements()?
            .iter()
            .map(workload)
            .collect::<Result<_, String>>()?,
    })
}

/// An object of strings as pairs.
fn pairs(value: &Value<'_>) -> Result<Vec<(String, String)>, String> {
    value
        .members()?
        .into_iter()
        .map(|(key, value)| Ok((key, value.text()?)))
        .collect()
}

/// The machine.
fn machine(value: &Value<'_>) -> Result<Machine, String> {
    let optional = |key: &str| value.optional(key).map(|v| v.text()).transpose();
    Ok(Machine {
        model: optional("model")?,
        cpu: value.text_of("cpu")?,
        cores: value.count_of("cores")?,
        threads: value.count_of("threads")?,
        memory_gib: value.member("memory_gib")?.number()?,
        os: value.text_of("os")?,
        kernel: value.text_of("kernel")?,
        affinity: value.text_of("affinity")?,
        confinement: optional("confinement")?,
        governor: optional("governor")?,
        power: optional("power")?,
        rustc: value.text_of("rustc")?,
        revision: value.text_of("revision")?,
    })
}

/// How often things ran.
fn plan(value: &Value<'_>) -> Result<Plan, String> {
    let sampling = |key: &str| -> Result<Sampling, String> {
        let s = value.member(key)?;
        Ok(Sampling {
            min_runs: s.count_of("min_runs")?,
            min_seconds: s.member("min_seconds")?.number()?,
            max_runs: s.count_of("max_runs")?,
        })
    };
    Ok(Plan {
        warm: sampling("warm")?,
        cold: sampling("cold")?,
        startup: sampling("startup")?,
    })
}

/// One workload.
fn workload(value: &Value<'_>) -> Result<WorkloadResult, String> {
    let tier = value.text_of("tier")?;
    let guard = value.member("guard")?;
    let guarded = |i: Implementation| column(&guard.member(i.key())?, guarded);
    Ok(WorkloadResult {
        name: value.text_of("name")?,
        tier: Tier::from_key(&tier).ok_or_else(|| value.error(&format!("has tier {tier:?}")))?,
        format: value.text_of("format")?,
        calls: value.optional("calls").map(|c| c.count()).transpose()?,
        guard: Guard {
            python: guarded(Implementation::Python)?,
            rust: guarded(Implementation::Rust)?,
        },
        warm: per_implementation(&value.member("warm")?, warm)?,
        cold: per_implementation(&value.member("cold")?, cold)?,
    })
}

/// The columns of the three implementations in `value`, read by `read`.
fn per_implementation<T>(
    value: &Value<'_>,
    read: fn(&Value<'_>) -> Result<T, String>,
) -> Result<PerImplementation<Column<T>>, String> {
    let of = |i: Implementation| column(&value.member(i.key())?, read);
    Ok(PerImplementation {
        matplotlib: of(Implementation::Matplotlib)?,
        python: of(Implementation::Python)?,
        rust: of(Implementation::Rust)?,
    })
}

/// A column: why it has no value, or the value `read` reads.
fn column<T>(
    value: &Value<'_>,
    read: fn(&Value<'_>) -> Result<T, String>,
) -> Result<Column<T>, String> {
    if let Some(why) = value.optional("not_supported") {
        return Ok(Column::NotSupported(why.text()?));
    }
    if let Some(why) = value.optional("not_comparable") {
        return Ok(Column::NotComparable(why.text()?));
    }
    read(value).map(Column::Measured)
}

/// The times of member `key`.
fn samples(value: &Value<'_>, key: &str) -> Result<Samples, String> {
    value
        .json
        .get(key)
        .and_then(Json::as_f64_vec)
        .and_then(Samples::new)
        .ok_or_else(|| value.error(&format!("has no finite times in {key:?}")))
}

/// Warm runs.
fn warm(value: &Value<'_>) -> Result<WarmResult, String> {
    Ok(WarmResult {
        seconds: samples(value, "seconds")?,
        bytes: value.count_of("bytes")?,
    })
}

/// Cold runs.
fn cold(value: &Value<'_>) -> Result<ColdResult, String> {
    let peaks_kib = value
        .member("peak_kib")?
        .elements()?
        .iter()
        .map(Value::count)
        .collect::<Result<Vec<u64>, String>>()?;
    if peaks_kib.is_empty() {
        return Err(value.error("has no peak memory"));
    }
    Ok(ColdResult {
        seconds: samples(value, "seconds")?,
        peaks_kib,
    })
}

/// The guard's verdicts on a rendering.
fn guarded(value: &Value<'_>) -> Result<Guarded, String> {
    Ok(Guarded {
        pixels: verdict(&value.member("pixels")?)?,
        structure: value
            .optional("structure")
            .map(|s| verdict(&s))
            .transpose()?,
    })
}

/// A verdict.
fn verdict(value: &Value<'_>) -> Result<Verdict, String> {
    if let Some(measure) = value.optional("same") {
        Ok(Verdict::Same(measure.number()?))
    } else if let Some(why) = value.optional("different") {
        Ok(Verdict::Different(why.text()?))
    } else if let Some(why) = value.optional("incomparable") {
        Ok(Verdict::Incomparable(why.text()?))
    } else {
        Err(value.error("is not a verdict"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_raw_data_is_refused_with_its_place() {
        assert!(from_text("[]").is_err());
        let err = from_text("{\"format\": \"other\"}").expect_err("another layout");
        assert!(err.contains("layout"), "{err}");
        let text = super::super::to_text(&super::super::tests::sample());
        let broken = text.replacen("\"tier\": \"small\"", "\"tier\": \"huge\"", 1);
        let err = from_text(&broken).expect_err("an unknown tier");
        assert!(err.contains("workloads[0]"), "{err}");
    }
}
