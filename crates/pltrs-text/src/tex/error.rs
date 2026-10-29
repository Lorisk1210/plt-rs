//! Why TeX text could not be typeset or drawn.

use std::fmt::{self, Write as _};

/// The kind of file TeX machinery read when it found something wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexFile {
    /// A DVI file, as `latex` writes it.
    Dvi,
    /// A virtual font (`.vf`).
    Vf,
    /// TeX font metrics (`.tfm`).
    Tfm,
    /// A font map (`pdftex.map`, `psfonts.map`).
    FontMap,
    /// A PostScript encoding (`.enc`).
    Encoding,
    /// The PNG image `dvipng` writes.
    Png,
    /// A Type 1 font a DVI file uses.
    Type1,
}

impl fmt::Display for TexFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dvi => "dvi file",
            Self::Vf => "vf file",
            Self::Tfm => "tfm file",
            Self::FontMap => "font map",
            Self::Encoding => "encoding file",
            Self::Png => "dvipng image",
            Self::Type1 => "Type 1 font",
        })
    }
}

/// Why TeX text could not be typeset or drawn (the errors matplotlib's
/// `texmanager` and `dviread` raise).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TexError {
    /// A program usetex runs is not installed (matplotlib's "could not be
    /// found" `RuntimeError`).
    Missing {
        /// The program (`latex`, `dvipng`).
        program: String,
    },
    /// A program failed on the text (matplotlib's "was not able to
    /// process the following string" `RuntimeError`).
    Failed {
        /// The program.
        program: String,
        /// The text given to TeX.
        tex: String,
        /// The command line, quoted as a shell would need it.
        command: String,
        /// What the program printed.
        output: String,
    },
    /// A file in the TeX cache could not be read or written.
    Io {
        /// The file or directory.
        path: String,
        /// What went wrong.
        message: String,
    },
    /// `kpsewhich` found no file of this name in the texmf tree.
    NotFound {
        /// The file name looked for.
        name: String,
    },
    /// The font map has no PostScript font for a TeX font.
    UnmappedFont {
        /// The font map.
        map: String,
        /// The TeX font.
        texname: String,
    },
    /// The font map names no Type 1 file for a TeX font.
    NoType1Font {
        /// The PostScript name of the font.
        psname: String,
        /// The TeX font.
        texname: String,
    },
    /// A file TeX wrote or reads is malformed.
    Malformed {
        /// What kind of file.
        file: TexFile,
        /// What is wrong with it.
        message: String,
    },
    /// A program usetex needs is not installed, found before typesetting
    /// anything (the warnings of matplotlib's `checkdep_usetex`).
    NotInstalled {
        /// What is missing (`TeX`, `LaTeX`, `dvipng`, and for PostScript
        /// output `dvips`, `ghostscript` and `the psfrag package`).
        program: String,
    },
    /// The font map gives a TeX font a PK bitmap font, which cannot be
    /// drawn as outlines.
    PkFont {
        /// The font file.
        path: String,
    },
    /// A DVI file uses XeTeX's native fonts, which matplotlib's `dviread`
    /// does not read either.
    XetexNativeFonts,
    /// A DVI file uses LuaTeX's native fonts, which matplotlib's `dviread`
    /// does not read either.
    LuatexNativeFonts,
}

impl TexError {
    /// A malformed `file`.
    pub fn malformed(file: TexFile, message: impl Into<String>) -> Self {
        Self::Malformed {
            file,
            message: message.into(),
        }
    }

    /// An I/O error on `path`.
    pub(crate) fn io(path: &std::path::Path, err: &std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            message: err.to_string(),
        }
    }
}

/// Python's `repr(s.encode("unicode_escape"))`: how matplotlib quotes the
/// string TeX failed on.
fn unicode_escape_repr(text: &str) -> String {
    let mut escaped = Vec::new();
    for c in text.chars() {
        match c {
            '\\' => escaped.extend_from_slice(b"\\\\"),
            '\t' => escaped.extend_from_slice(b"\\t"),
            '\n' => escaped.extend_from_slice(b"\\n"),
            '\r' => escaped.extend_from_slice(b"\\r"),
            ' '..='~' => escaped.push(c as u8),
            c if u32::from(c) < 0x100 => {
                escaped.extend_from_slice(format!("\\x{:02x}", u32::from(c)).as_bytes());
            }
            c if u32::from(c) < 0x1_0000 => {
                escaped.extend_from_slice(format!("\\u{:04x}", u32::from(c)).as_bytes());
            }
            c => escaped.extend_from_slice(format!("\\U{:08x}", u32::from(c)).as_bytes()),
        }
    }
    bytes_repr(&escaped)
}

/// Python's `repr(bytes)`.
pub(crate) fn bytes_repr(data: &[u8]) -> String {
    let quote = if data.contains(&b'\'') && !data.contains(&b'"') {
        '"'
    } else {
        '\''
    };
    let mut out = format!("b{quote}");
    for &byte in data {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\t' => out.push_str("\\t"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b if char::from(b) == quote => {
                out.push('\\');
                out.push(quote);
            }
            b' '..=b'~' => out.push(char::from(byte)),
            _ => {
                let _ = write!(out, "\\x{byte:02x}");
            }
        }
    }
    out.push(quote);
    out
}

impl fmt::Display for TexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { program } => write!(
                f,
                "Failed to process string with tex because {program} could not be found"
            ),
            Self::Failed {
                program,
                tex,
                command,
                output,
            } => write!(
                f,
                "{program} was not able to process the following string:\n{}\n\n\
                 Here is the full command invocation and its output:\n\n{command}\n\n{output}\n\n",
                unicode_escape_repr(tex)
            ),
            Self::Io { path, message } => write!(f, "TeX cache file {path}: {message}"),
            Self::NotFound { name } => write!(
                f,
                "pltrs' TeX implementation searched for a file named {name:?} in your texmf \
                 tree, but could not find it"
            ),
            Self::UnmappedFont { map, texname } => write!(
                f,
                "The font map {map:?} is missing a PostScript font associated to TeX font \
                 {texname:?}; this problem can often be solved by installing a suitable \
                 PostScript font package in your TeX package manager"
            ),
            Self::NoType1Font { psname, texname } => write!(
                f,
                "No usable font file found for {psname} ({texname}); the font may lack a \
                 Type-1 version"
            ),
            Self::Malformed { file, message } => write!(f, "malformed {file}: {message}"),
            Self::NotInstalled { program } => write!(f, "usetex mode requires {program}."),
            Self::PkFont { path } => {
                write!(f, "{path} is a PK bitmap font, which cannot be drawn")
            }
            Self::XetexNativeFonts => {
                f.write_str("DVI files with XeTeX native fonts are not supported")
            }
            Self::LuatexNativeFonts => {
                f.write_str("DVI files with LuaTeX native fonts are not supported")
            }
        }
    }
}

impl std::error::Error for TexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_quote_the_text_as_python_does() {
        // repr("é\\frac{1}{2}\n'".encode("unicode_escape"))
        assert_eq!(
            unicode_escape_repr("\u{e9}\\frac{1}{2}\n'"),
            r#"b"\\xe9\\\\frac{1}{2}\\n'""#
        );
        assert_eq!(unicode_escape_repr("\u{2603}"), r"b'\\u2603'");
        let failed = TexError::Failed {
            program: "latex".into(),
            tex: "$x".into(),
            command: "latex file.tex".into(),
            output: "! Missing $ inserted.".into(),
        };
        assert_eq!(
            failed.to_string(),
            "latex was not able to process the following string:\nb'$x'\n\n\
             Here is the full command invocation and its output:\n\n\
             latex file.tex\n\n! Missing $ inserted.\n\n"
        );
    }

    #[test]
    fn missing_programs_are_named() {
        let missing = TexError::Missing {
            program: "dvipng".into(),
        };
        assert_eq!(
            missing.to_string(),
            "Failed to process string with tex because dvipng could not be found"
        );
        let not_installed = TexError::NotInstalled {
            program: "dvipng".into(),
        };
        assert_eq!(not_installed.to_string(), "usetex mode requires dvipng.");
    }
}
