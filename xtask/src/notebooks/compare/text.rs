//! Text as the comparison reads it: what differs from one run to the next
//! whatever the library (object addresses, random ids, the files the
//! kernel compiles cells into, the venv's directory of packages) is
//! replaced, and so is what pltrs is by design: its module names, which
//! the `matplotlib` distribution of pltrs makes aliases of matplotlib's
//! (`matplotlib.lines` is `pltrs.lines`, so a repr names
//! `pltrs.lines.Line2D` where matplotlib's names `matplotlib.lines.Line2D`),
//! and its own backend, `pltrs` (its windows), which lists of the valid
//! backends name besides matplotlib's.

/// The random ids one record's text uses (comm ids, widget models, the
/// ids of output areas and animations), numbered in the order they first
/// appear, so that two records compare their ids by where they appear.
#[derive(Clone, Debug, Default)]
pub(super) struct Names {
    /// Every id met, in order.
    ids: Vec<String>,
}

impl Names {
    /// The number of `id`, given it if it is new.
    fn number(&mut self, id: &str) -> usize {
        if let Some(index) = self.ids.iter().position(|known| known == id) {
            return index;
        }
        self.ids.push(id.to_owned());
        self.ids.len() - 1
    }
}

/// pltrs' module prefixes and matplotlib's names of them, the longest
/// first.
const MODULES: [(&str, &str); 2] = [
    ("pltrs.mpl_toolkits.", "mpl_toolkits."),
    ("pltrs.", "matplotlib."),
];

/// pltrs' own backend as a list of backends names it.
const PLTRS_BACKEND: [&str; 2] = ["'pltrs', ", ", 'pltrs'"];

/// The directory ipykernel compiles a cell into, before its process id.
const CELL_DIRECTORY: &str = "ipykernel_";

/// `text` as the comparison reads it (see the module's description),
/// numbering its ids in `names`.
pub(super) fn normalized(text: &str, names: &mut Names) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut boundary = true;
    let mut hex_before = false;
    while let Some(next) = rest.chars().next() {
        let replacement = if boundary { replaced(rest) } else { None };
        let id = (!hex_before)
            .then(|| id_length(rest))
            .flatten()
            .map(|length| (format!("<id{}>", names.number(&rest[..length])), length));
        if let Some((replacement, length)) = id.or(replacement) {
            out.push_str(&replacement);
            rest = &rest[length..];
            boundary = true;
            hex_before = false;
            continue;
        }
        out.push(next);
        rest = &rest[next.len_utf8()..];
        // An underscore ends a word here: `IPY_MODEL_<id>` names an id.
        boundary = !next.is_ascii_alphanumeric();
        hex_before = is_hex(next);
    }
    out
}

/// Whether `c` continues a word (an identifier or a number).
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// What replaces the start of `text`, which follows a word boundary, and
/// the length it replaces, if anything does.
fn replaced(text: &str) -> Option<(String, usize)> {
    if let Some(hex) = text.strip_prefix("0x") {
        let digits = hex.len()
            - hex
                .trim_start_matches(|c: char| c.is_ascii_hexdigit())
                .len();
        if digits > 0 && !hex[digits..].starts_with(is_word) {
            return Some(("0x…".to_owned(), 2 + digits));
        }
    }
    if let Some(length) = cell_file_length(text) {
        return Some(("<cell>".to_owned(), length));
    }
    if let Some(length) = site_packages_length(text) {
        return Some(("<site-packages>".to_owned(), length));
    }
    if let Some(entry) = PLTRS_BACKEND.iter().find(|entry| text.starts_with(**entry)) {
        return Some((String::new(), entry.len()));
    }
    MODULES.iter().find_map(|(module, name)| {
        text.starts_with(module)
            .then(|| ((*name).to_owned(), module.len()))
    })
}

/// Whether `c` is a lowercase hex digit.
fn is_hex(c: char) -> bool {
    c.is_ascii_digit() || ('a'..='f').contains(&c)
}

/// The length of the random id `text` starts with, which no hex digit
/// precedes: 32 lowercase hex digits (`uuid4().hex`, also after a word,
/// as in matplotlib's `_anim_img<id>`) or a UUID's groups of 8, 4, 4, 4
/// and 12, not followed by more of a word.
fn id_length(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let hex_run = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| is_hex(char::from(**b)))
            .count()
    };
    let ends_word = |at: usize| !text[at..].starts_with(is_word);
    if hex_run(0) == 32 && ends_word(32) {
        return Some(32);
    }
    let mut at = 0;
    for (index, group) in [8, 4, 4, 4, 12].into_iter().enumerate() {
        if index > 0 {
            if bytes.get(at) != Some(&b'-') {
                return None;
            }
            at += 1;
        }
        if hex_run(at) != group {
            return None;
        }
        at += group;
    }
    ends_word(at).then_some(at)
}

/// The length of the directory of installed packages a path `text`
/// starts with names (`/.../site-packages`), which is another venv's in
/// each run.
fn site_packages_length(text: &str) -> Option<usize> {
    const SITE_PACKAGES: &str = "/site-packages";
    if !text.starts_with('/') {
        return None;
    }
    let path = text
        .split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '(' | ')'))
        .next()?;
    let at = path.find(&format!("{SITE_PACKAGES}/"))?;
    Some(at + SITE_PACKAGES.len())
}

/// The length of the cell file `text` starts with:
/// `ipykernel_<process>/<hash>.py`.
fn cell_file_length(text: &str) -> Option<usize> {
    let rest = text.strip_prefix(CELL_DIRECTORY)?;
    let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let process = digits(rest);
    let rest = rest[process..].strip_prefix('/')?;
    let hash = digits(rest);
    rest[hash..].strip_prefix(".py")?;
    (process > 0 && hash > 0).then_some(CELL_DIRECTORY.len() + process + 1 + hash + 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal(text: &str) -> String {
        normalized(text, &mut Names::default())
    }

    #[test]
    fn addresses_are_replaced() {
        assert_eq!(
            normal("[<matplotlib.lines.Line2D at 0x7f3a2b1c>]"),
            "[<matplotlib.lines.Line2D at 0x…>]"
        );
        assert_eq!(normal("0x"), "0x");
        assert_eq!(normal("a0x12"), "a0x12");
    }

    #[test]
    fn ids_are_numbered_where_they_first_appear() {
        let a = "0123456789abcdef0123456789abcdef";
        let b = "12345678-1234-1234-1234-123456789abc";
        let text = format!("IPY_MODEL_{a} <div id='{b}'> {a} {a}0 img{a} f{a}");
        assert_eq!(
            normal(&text),
            format!("IPY_MODEL_<id0> <div id='<id1>'> <id0> {a}0 img<id0> f{a}")
        );
        let mut names = Names::default();
        assert_eq!(normalized(b, &mut names), "<id0>");
        assert_eq!(normalized(a, &mut names), "<id1>");
        assert_eq!(normalized(b, &mut names), "<id0>");
    }

    #[test]
    fn cell_files_are_replaced() {
        assert_eq!(
            normal("/tmp/ipykernel_4242/1234567.py:3: UserWarning: x"),
            "/tmp/<cell>:3: UserWarning: x"
        );
        assert_eq!(normal("ipykernel_/1.py"), "ipykernel_/1.py");
    }

    #[test]
    fn installed_packages_are_replaced() {
        assert_eq!(
            normal(
                "cannot import x (/h/target/notebooks/venv/lib/python3.14/site-packages/IPython/display.py)"
            ),
            "cannot import x (<site-packages>/IPython/display.py)"
        );
        assert_eq!(normal("/a/site-packages"), "/a/site-packages");
    }

    #[test]
    fn pltrs_modules_read_as_matplotlib() {
        assert_eq!(
            normal("<pltrs.lines.Line2D at 0x1f> pltrs.mpl_toolkits.mplot3d"),
            "<matplotlib.lines.Line2D at 0x…> mpl_toolkits.mplot3d"
        );
        assert_eq!(normal("mypltrs.x"), "mypltrs.x");
        assert_eq!(
            normal("['pgf', 'pltrs', 'ps'] ['svg', 'pltrs']"),
            "['pgf', 'ps'] ['svg']"
        );
        assert_eq!(normal("Grüße pltrs."), "Grüße matplotlib.");
    }
}
