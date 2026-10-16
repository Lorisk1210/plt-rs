"""Generate crates/plt/src/rc/table.rs from matplotlib's rcParams definition.

Run with the reference venv:  .venv/bin/python -I crates/plt/tools/gen_rcparams.py
The output is committed; this script only needs re-running when matplotlib
adds or changes parameters.
"""

import pathlib
import re

import matplotlib as mpl
import matplotlib.rcsetup as rcsetup

OUT = pathlib.Path(__file__).resolve().parents[1] / "src" / "rc" / "table.rs"
SKIP = {"_internal.classic_mode", "backend"}

# validator name -> (Rust type, extra validation expression or None)
TYPES = {
    "validate_bool": ("bool", None),
    "validate_float": ("f64", None),
    "_validate_greaterequal0_lessequal1": ("f64", "validate::unit_interval()"),
    "_validate_greaterthan_minushalf": ("f64", "validate::above_minus_half()"),
    "validate_float_or_None": ("Option<f64>", None),
    "validate_int": ("i64", None),
    "validate_int_or_None": ("Option<i64>", None),
    "validate_floatlist": ("Vec<f64>", None),
    "validate_intlist": ("Vec<i64>", None),
    "validate_str": ("String", None),
    "validate_str_or_None": ("Option<String>", None),
    "validate_strlist": ("Vec<String>", None),
    "validate_anylist": ("Vec<String>", None),
    "validate_color": ("Color", None),
    "validate_color_or_auto": ("ColorOrAuto", None),
    "validate_color_or_inherit": ("ColorOrInherit", None),
    "_validate_color_or_None": ("Option<Color>", None),
    "_validate_color_or_edge": ("ColorOrEdge", None),
    "_validate_color_or_linecolor": ("LabelColor", None),
    "CapStyle": ("CapStyle", None),
    "JoinStyle": ("JoinStyle", None),
    "_validate_linestyle": ("LineStyle", None),
    "_validate_linestyle_or_None": ("Option<LineStyle>", None),
    "_validate_marker": ("Marker", None),
    "validate_fontsize": ("FontSize", None),
    "validate_fontsize_None": ("Option<FontSize>", None),
    "validate_fontweight": ("FontWeight", None),
    "validate_fontstretch": ("FontStretch", None),
    "validate_font_properties": ("String", None),
    "validate_fonttype": ("FontType", None),
    "validate_cycler": ("Cycler", None),
    "validate_sketch": ("Option<[f64; 3]>", None),
    "validate_whiskers": ("Whiskers", None),
    "validate_hist_bins": ("HistBins", None),
    "validate_dpi": ("Dpi", None),
    "validate_aspect": ("Aspect", None),
    "validate_axisbelow": ("AxisBelow", None),
    "_validate_legend_loc": ("String", "validate::legend_loc"),
    "_validate_minor_tick_ndivs": ("String", "validate::minor_ndivs"),
    "validate_bbox": ("Option<String>", 'validate::optional_one_of(&["tight", "standard"])'),
    "_validate_date": ("String", None),
    "_validate_pathlike": ("String", None),
    "_validate_toolbar": ("String", 'validate::one_of(&["None", "toolbar2", "toolmanager"])'),
    "validate_ps_distiller": ("Distiller", None),
    "_validate_mathtext_fallback": ("Option<String>", 'validate::optional_one_of(&["cm", "stix", "stixsans"])'),
    "_validate_cmap": ("String", None),
}


def rc_file_entries():
    """`key -> (value text, comment)` from the documented matplotlibrc."""
    text = pathlib.Path(mpl.get_data_path(), "matplotlibrc").read_text()
    entries = {}
    for line in text.splitlines():
        m = re.match(r"^#([a-z][\w.\-]*)\s*:\s*(.*)$", line)
        if not m:
            continue
        key, rest = m.groups()
        value, _, comment = split_comment(rest)
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] == '"':
            value = value[1:-1]  # matplotlib's rc reader unwraps double quotes
        entries.setdefault(key, (value, comment.strip()))
    return entries


def split_comment(text):
    """Split at the first '#' outside quotes (as matplotlib's _strip_comment)."""
    quote = None
    for i, ch in enumerate(text):
        if ch in "'\"":
            quote = None if quote == ch else (quote or ch)
        elif ch == "#" and quote is None:
            return text[:i], "#", text[i + 1:]
    return text, "", ""


def field_name(key):
    return re.sub(r"[.\-]", "_", key)


def rust_str(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


CODE_WORD = re.compile(
    r"''(.+?)''"                       # reST literals
    r"|([A-Za-z_][\w.\[\]|]*\(\))"    # calls like show()
    r"|\b([a-z_]+(?:\.[a-z_0-9]+)+)\b"  # dotted names like axes.prop_cycle
    r"|\b([A-Z][a-z]+[A-Z]\w*)\b"       # CamelCase identifiers
)


def doc(comment, key):
    comment = re.sub(r"\s+", " ", comment).strip(" .")
    if not comment:
        return f"`{key}`."
    comment = comment[0].upper() + comment[1:]
    comment = comment.replace("`", "'")
    comment = CODE_WORD.sub(lambda m: f"`{next(g for g in m.groups() if g)}`", comment)
    return f"`{key}`: {comment}."


def main():
    entries = rc_file_entries()
    rows = []
    for key, validator in rcsetup._validators.items():
        if key in SKIP:
            continue
        name = getattr(validator, "__name__", None) or type(validator).__name__
        value, comment = entries[key]
        if name == "ValidateInStrings":
            allowed = sorted(validator.valid.values()) if isinstance(validator.valid, dict) else sorted(validator.valid)
            fn = "one_of_ignore_case" if validator.ignorecase else "one_of"
            rust_type, check = "String", f"validate::{fn}(&[{', '.join(rust_str(a) for a in allowed)}])"
        else:
            rust_type, check = TYPES[name]
        rows.append((key, rust_type, value, check, doc(comment, key)))

    lines = [
        "//! Every rcParam with its type, default and validation.",
        "//!",
        "//! Generated by `crates/plt/tools/gen_rcparams.py` from matplotlib's",
        "//! `rcsetup` validators and documented `matplotlibrc` defaults.",
        "",
        "use super::cycler::Cycler;",
        "use super::kinds::{",
        "    Aspect, AxisBelow, ColorOrAuto, ColorOrEdge, ColorOrInherit, Distiller, Dpi, FontType,",
        "    HistBins, LabelColor, Whiskers,",
        "};",
        "use super::validate;",
        "use crate::colors::Color;",
        "use crate::fonts::{FontSize, FontStretch, FontWeight};",
        "use crate::lines::LineStyle;",
        "use crate::markers::Marker;",
        "use plt_render::{CapStyle, JoinStyle};",
        "",
        "rc_params! {",
    ]
    for key, rust_type, value, check, docline in rows:
        lines.append(f"    /// {docline}")
        check_part = f" where {check}" if check else ""
        lines.append(f"    {field_name(key)}: {rust_type} = {rust_str(value)} => {rust_str(key)}{check_part};")
    lines.append("}")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {len(rows)} parameters to {OUT}")


if __name__ == "__main__":
    main()
