//! How Ghostscript names itself in the `%%Invocation` comment of the
//! documents it distills: by how the host installed and started it, which
//! neither matplotlib nor pltrs decides, so that it is compared as `gs`.

/// The start of the comment.
const INVOCATION: &str = "%%Invocation: ";

/// What Ghostscript writes before the program when it was started by an
/// absolute path (the goldens' recorder writes the path so).
const BY_PATH: &str = "path/";

/// The names of Ghostscript's console program on Windows.
const WINDOWS_PROGRAMS: [&str; 2] = ["gswin64c", "gswin32c"];

/// The arguments Ghostscript's Windows build puts before those it was
/// given: the display's format and resolution.
const WINDOWS_DISPLAY_ARGUMENTS: [&str; 2] = ["-dDisplayFormat=", "-dDisplayResolution="];

/// The header comment `line` with Ghostscript's program named `gs` however
/// the host started it: by name, by an absolute path (`path/gs`), or on
/// Windows as `gswin64c` with the display's arguments its build adds.
/// Other lines are returned as they are.
pub(crate) fn invoked_as_gs(line: &str) -> String {
    let Some(command) = line.strip_prefix(INVOCATION) else {
        return line.to_owned();
    };
    let command = command.strip_prefix(BY_PATH).unwrap_or(command);
    let mut words = command.split(' ');
    match words.next() {
        Some(program) if WINDOWS_PROGRAMS.contains(&program) => {
            let given: Vec<&str> = words
                .skip_while(|word| {
                    WINDOWS_DISPLAY_ARGUMENTS
                        .iter()
                        .any(|display| word.starts_with(display))
                })
                .collect();
            format!("{INVOCATION}gs {}", given.join(" "))
        }
        _ => format!("{INVOCATION}{command}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_is_gs_however_started() {
        let given = "%%Invocation: gs -dBATCH -r6000 -sOutputFile=? ?";
        assert_eq!(invoked_as_gs(given), given);
        assert_eq!(
            invoked_as_gs("%%Invocation: path/gs -dBATCH -r6000 -sOutputFile=? ?"),
            given
        );
        assert_eq!(
            invoked_as_gs(
                "%%Invocation: gswin64c -dDisplayFormat=198788 -dDisplayResolution=96 \
                 -dBATCH -r6000 -sOutputFile=? ?"
            ),
            given
        );
    }

    #[test]
    fn other_lines_and_programs_stay() {
        assert_eq!(invoked_as_gs("%%Pages: 1"), "%%Pages: 1");
        assert_eq!(
            invoked_as_gs("%%Invocation: path/pdftops -eps"),
            "%%Invocation: pdftops -eps"
        );
        assert_eq!(
            invoked_as_gs("%%Invocation: gswin64c -dBATCH"),
            "%%Invocation: gs -dBATCH"
        );
    }
}
