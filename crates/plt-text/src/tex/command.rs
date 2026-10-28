//! Running the TeX programs (matplotlib's
//! `TexManager._run_checked_subprocess` and `cbook._pformat_subprocess`).

use std::ffi::OsStr;
use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};

use super::error::TexError;

/// `shlex.quote`: `arg` as a POSIX shell reads it back.
fn shell_quote(arg: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c);
    if !arg.is_empty() && arg.chars().all(safe) {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', r#"'"'"'"#))
    }
}

/// `cbook._pformat_subprocess`: the command line, quoted.
pub(crate) fn format_command(program: &str, args: &[&OsStr]) -> String {
    std::iter::once(shell_quote(program))
        .chain(args.iter().map(|arg| shell_quote(&arg.to_string_lossy())))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `program` can be run: an executable file of that name is on
/// `PATH` (`shutil.which`).
pub(crate) fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(program);
            std::fs::metadata(&candidate).is_ok_and(|meta| meta.is_file() && is_executable(&meta))
        })
    })
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &std::fs::Metadata) -> bool {
    true
}

/// Runs `program` with `args` in `cwd`, its output and errors collected
/// together, and fails as matplotlib does when it cannot be started or
/// exits with an error (the message quotes `tex`, the string being
/// processed). Returns what it printed.
pub(crate) fn run_checked(
    program: &str,
    args: &[&OsStr],
    tex: &str,
    cwd: &Path,
) -> Result<Vec<u8>, TexError> {
    let missing = || TexError::Missing {
        program: program.to_owned(),
    };
    let (mut reader, writer) = std::io::pipe().map_err(|_| missing())?;
    let stderr = writer.try_clone().map_err(|_| missing())?;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(stderr);
    let mut child = command.spawn().map_err(|_| missing())?;
    // The pipe's write ends live in `command`; drop them so that the read
    // ends when the program exits.
    drop(command);
    let mut output = Vec::new();
    let read = reader.read_to_end(&mut output);
    let status = child.wait().map_err(|_| missing())?;
    if read.is_err() || !status.success() {
        return Err(TexError::Failed {
            program: program.to_owned(),
            tex: tex.to_owned(),
            command: format_command(program, args),
            output: String::from_utf8_lossy(&output).into_owned(),
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_quoted_like_shlex() {
        let args = [
            OsStr::new("-interaction=nonstopmode"),
            OsStr::new("a b"),
            OsStr::new("it's"),
            OsStr::new(""),
        ];
        assert_eq!(
            format_command("latex", &args),
            r#"latex -interaction=nonstopmode 'a b' 'it'"'"'s' ''"#
        );
    }

    #[test]
    fn missing_programs_and_failures_are_errors() {
        let cwd = std::env::temp_dir();
        assert_eq!(
            run_checked("plt-rs-no-such-program", &[], "x", &cwd),
            Err(TexError::Missing {
                program: "plt-rs-no-such-program".into()
            })
        );
        assert!(!on_path("plt-rs-no-such-program"));
        if on_path("sh") {
            let args = [
                OsStr::new("-c"),
                OsStr::new("echo out; echo err >&2; exit 3"),
            ];
            match run_checked("sh", &args, "x", &cwd) {
                Err(TexError::Failed { output, .. }) => assert_eq!(output, "out\nerr\n"),
                other => panic!("{other:?}"),
            }
            let args = [OsStr::new("-c"), OsStr::new("echo fine")];
            assert_eq!(
                run_checked("sh", &args, "x", &cwd).as_deref(),
                Ok(&b"fine\n"[..])
            );
        }
    }
}
