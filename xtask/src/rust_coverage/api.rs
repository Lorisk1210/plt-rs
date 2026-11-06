//! matplotlib's public callables, listed by `spec/rust-coverage/api.py` in
//! the reference venv: the list `cargo xtask diff --coverage` measures
//! the differential tester's programs on.

use std::process::Command;

use crate::python::reference_python;

/// One public callable of matplotlib.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Callable {
    /// Its module (`matplotlib.axes`).
    pub(super) module: String,
    /// Its key within the module: `Owner.name` for a class' members,
    /// `name` at module level.
    pub(super) key: String,
    /// What it is: function, class, method, property, classmethod or
    /// staticmethod.
    pub(super) kind: String,
    /// Whether matplotlib deprecates it.
    pub(super) deprecated: bool,
}

impl Callable {
    /// Its group for reports: the class it belongs to
    /// (`matplotlib.axes:Axes`), or the module for module-level names.
    pub(super) fn group(&self) -> String {
        match self.key.split_once('.') {
            Some((owner, _)) => format!("{}:{owner}", self.module),
            None => self.module.clone(),
        }
    }

    /// Its name, without the class.
    pub(super) fn name(&self) -> &str {
        self.key.rsplit('.').next().unwrap_or(&self.key)
    }
}

/// The callables, from `api.py` run in the reference venv.
pub(super) fn callables() -> Result<Vec<Callable>, String> {
    let script = crate::python::root().join("spec/rust-coverage/api.py");
    let python = reference_python();
    let output = Command::new(&python)
        .arg("-I")
        .arg(&script)
        .env("MPLBACKEND", "agg")
        .output()
        .map_err(|err| format!("cannot run {}: {err}", python.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} failed ({}):\n{}",
            script.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    parse(&String::from_utf8_lossy(&output.stdout))
}

/// The callables of `api.py`'s output.
fn parse(text: &str) -> Result<Vec<Callable>, String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let (Some(full), Some(kind), deprecated) =
                (fields.next(), fields.next(), fields.next())
            else {
                return Err(format!("api.py printed a malformed line: {line:?}"));
            };
            let (module, key) = full
                .split_once(':')
                .ok_or_else(|| format!("api.py printed a key without a module: {full:?}"))?;
            Ok(Callable {
                module: module.to_owned(),
                key: key.to_owned(),
                kind: kind.to_owned(),
                deprecated: deprecated == Some("deprecated"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_give_the_module_key_kind_and_deprecation() {
        let callables = parse(
            "matplotlib.axes:Axes.hist\tmethod\t\nmatplotlib:sanitize_sequence\tfunction\tdeprecated\n",
        )
        .expect("well formed");
        assert_eq!(callables[0].module, "matplotlib.axes");
        assert_eq!(callables[0].key, "Axes.hist");
        assert_eq!(callables[0].group(), "matplotlib.axes:Axes");
        assert_eq!(callables[0].name(), "hist");
        assert!(!callables[0].deprecated);
        assert_eq!(callables[1].group(), "matplotlib");
        assert!(callables[1].deprecated);
        assert!(parse("no tabs here\n").is_err());
        assert!(parse("nomodule\tfunction\t\n").is_err());
    }
}
