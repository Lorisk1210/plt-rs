"""Generate src/shape/ucd_tables.rs: general categories, bidi classes,
paired brackets, mirroring, Arabic joining types and the Arabic
presentation-form tables HarfBuzz's fallback shaper uses.

The data comes from the Unicode 17.0.0 character database, the version
libraqm's SheenBidi and HarfBuzz 14 (matplotlib 3.11) are built with.
Download the files into an empty directory and pass it as the argument:

    mkdir ucd && cd ucd
    for f in UnicodeData.txt ArabicShaping.txt BidiBrackets.txt \
             BidiMirroring.txt extracted/DerivedBidiClass.txt; do
        curl -sSLO https://www.unicode.org/Public/17.0.0/ucd/$f
    done
    cd .. && python3 -I crates/plt-font/tools/gen_ucd_tables.py ucd

The Arabic presentation-form tables reproduce HarfBuzz's
`gen-arabic-table.py` (`shaping_table`, `ligature_table`,
`ligature_mark_table`, `ligature_3_table`).
"""

import os.path
import pathlib
import sys

OUT = pathlib.Path(__file__).resolve().parent.parent / "src" / "shape" / "ucd_tables.rs"
MAX = 0x110000

# Order of `GeneralCategory` in src/shape/category.rs.
GENERAL_CATEGORIES = [
    "Cc", "Cf", "Cn", "Co", "Cs", "Ll", "Lm", "Lo", "Lt", "Lu", "Mc", "Me", "Mn", "Nd",
    "Nl", "No", "Pc", "Pd", "Pe", "Pf", "Pi", "Po", "Ps", "Sc", "Sk", "Sm", "So", "Zl",
    "Zp", "Zs",
]

# Order of `BidiClass` in src/shape/bidi/class.rs.
BIDI_CLASSES = [
    "L", "R", "AL", "EN", "ES", "ET", "AN", "CS", "NSM", "BN", "B", "S", "WS", "ON",
    "LRE", "LRO", "RLE", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI",
]
BIDI_LONG_NAMES = {
    "Left_To_Right": "L", "Right_To_Left": "R", "Arabic_Letter": "AL",
    "European_Number": "EN", "European_Separator": "ES", "European_Terminator": "ET",
    "Arabic_Number": "AN", "Common_Separator": "CS", "Nonspacing_Mark": "NSM",
    "Boundary_Neutral": "BN", "Paragraph_Separator": "B", "Segment_Separator": "S",
    "White_Space": "WS", "Other_Neutral": "ON",
}

# Joining type codes of `JoiningType` in src/shape/arabic/joining.rs
# (U is the default and not stored).
JOINING_CODES = {"L": 1, "R": 2, "D": 3, "C": 3, "ALAPH": 4, "DALATH_RISH": 5, "T": 6}

# HarfBuzz's subset of presentation-form ligatures used by the fallback shaper.
LIGATURES = (
    0xF2EE, 0xFC08, 0xFC0E, 0xFC12, 0xFC32, 0xFC3F, 0xFC40, 0xFC41, 0xFC42,
    0xFC44, 0xFC4E, 0xFC5E, 0xFC60, 0xFC61, 0xFC62, 0xFC6A, 0xFC6D, 0xFC6F,
    0xFC70, 0xFC73, 0xFC75, 0xFC86, 0xFC8F, 0xFC91, 0xFC94, 0xFC9C, 0xFC9D,
    0xFC9E, 0xFC9F, 0xFCA1, 0xFCA2, 0xFCA3, 0xFCA4, 0xFCA8, 0xFCAA, 0xFCAC,
    0xFCB0, 0xFCC9, 0xFCCA, 0xFCCB, 0xFCCC, 0xFCCD, 0xFCCE, 0xFCCF, 0xFCD0,
    0xFCD1, 0xFCD2, 0xFCD3, 0xFCD5, 0xFCDA, 0xFCDB, 0xFCDC, 0xFCDD, 0xFD30,
    0xFD88, 0xFEF5, 0xFEF6, 0xFEF7, 0xFEF8, 0xFEF9, 0xFEFA, 0xFEFB, 0xFEFC,
    0xF201, 0xF211, 0xF2EE,
)

# Private-use ligatures HarfBuzz adds to UnicodeData.txt.
PUA_LINES = [
    "F201;PUA ARABIC LIGATURE LELLAH ISOLATED FORM;Lo;0;AL;<isolated> 0644 0644 0647;;;;N;;;;;",
    "F211;PUA ARABIC LIGATURE LAM WITH MEEM WITH JEEM INITIAL FORM;Lo;0;AL;<initial> 0644 0645 062C;;;;N;;;;;",
    "F2EE;PUA ARABIC LIGATURE SHADDA WITH FATHATAN ISOLATED FORM;Lo;0;AL;<isolated> 0020 064B 0651;;;;N;;;;;",
]


def data_lines(path):
    """Non-comment lines of a UCD file split into stripped fields."""
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            yield [field.strip() for field in line.split(";")]


def code_range(text):
    first, _, last = text.partition("..")
    return int(first, 16), int(last or first, 16)


def unicode_data(ucd):
    """`(code, fields)` for every UnicodeData.txt entry, ranges expanded."""
    entries = []
    pending = None
    for line in (ucd / "UnicodeData.txt").read_text(encoding="utf-8").splitlines():
        fields = line.split(";")
        code = int(fields[0], 16)
        if fields[1].endswith(", First>"):
            pending = code
            continue
        if fields[1].endswith(", Last>"):
            entries.extend((c, fields) for c in range(pending, code + 1))
            continue
        entries.append((code, fields))
    return entries


def general_categories(entries):
    categories = ["Cn"] * MAX
    for code, fields in entries:
        categories[code] = fields[2]
    return categories


def bidi_classes(ucd):
    path = ucd / "DerivedBidiClass.txt"
    classes = [None] * MAX
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("# @missing:"):
            span, name = (f.strip() for f in line[len("# @missing:"):].split(";"))
            first, last = code_range(span)
            classes[first:last + 1] = [BIDI_LONG_NAMES[name]] * (last - first + 1)
    for fields in data_lines(path):
        first, last = code_range(fields[0])
        classes[first:last + 1] = [fields[1]] * (last - first + 1)
    return classes


def runs(values):
    """`(start, value)` for each maximal run of equal values."""
    out = []
    for code, value in enumerate(values):
        if not out or out[-1][1] != value:
            out.append((code, value))
    return out


def joining_types(ucd, categories):
    """`(first, last, code)` runs of non-U joining types (HarfBuzz's
    `get_joining_type`: unlisted Mn/Me/Cf characters are transparent)."""
    listed = {}
    for fields in data_lines(ucd / "ArabicShaping.txt"):
        code = int(fields[0], 16)
        group = fields[3]
        listed[code] = group.replace(" ", "_") if group in ("ALAPH", "DALATH RISH") else fields[2]
    values = []
    for code in range(MAX):
        kind = listed.get(code)
        if kind is None:
            kind = "T" if categories[code] in ("Mn", "Me", "Cf") else "U"
        values.append(JOINING_CODES.get(kind, 0))
    out = []
    for code, value in enumerate(values):
        if value == 0:
            continue
        if out and out[-1][1] == code - 1 and out[-1][2] == value:
            out[-1][1] = code
        else:
            out.append([code, code, value])
    return out


def brackets(ucd):
    out = []
    for fields in data_lines(ucd / "BidiBrackets.txt"):
        out.append((int(fields[0], 16), int(fields[1], 16), fields[2] == "o"))
    return sorted(out)


def mirrors(ucd):
    return sorted((int(f[0], 16), int(f[1], 16)) for f in data_lines(ucd / "BidiMirroring.txt"))


def presentation_forms(ucd):
    """HarfBuzz's `print_shaping_table`: forms per character and the
    ligature tables of the fallback shaper."""
    shapes, ligatures = {}, {}
    lines = (ucd / "UnicodeData.txt").read_text(encoding="utf-8").splitlines() + PUA_LINES
    for line in lines:
        fields = [x.strip() for x in line.split(";")]
        if fields[5][0:1] != "<":
            continue
        items = fields[5].split(" ")
        shape, items = items[0][1:-1], tuple(int(x, 16) for x in items[1:])
        code = int(fields[0], 16)
        if shape not in ("initial", "medial", "isolated", "final"):
            continue
        if len(items) != 1:
            if items[0] == 0x0020:
                items = items[:0:-1]
                shape = None
            if code not in LIGATURES:
                continue
            ligatures.setdefault(items, {})[shape] = code
        else:
            shapes.setdefault(items[0], {})[shape] = code
    forms = []
    for code in range(min(shapes), max(shapes) + 1):
        row = [shapes.get(code, {}).get(s, 0) for s in ("initial", "medial", "final", "isolated")]
        if any(row):
            forms.append((code, row))
    # Forms of the first, middle and last components for each ligature shape.
    component_forms = {
        "isolated": ("initial", "medial", "final"),
        "final": ("medial", "medial", "final"),
        "initial": ("initial", "medial", "medial"),
    }
    ligas_2, ligas_3, ligas_mark = {}, {}, {}
    for key, by_shape in ligatures.items():
        for shape, code in by_shape.items():
            if shape is None:
                ligas_mark.setdefault(key[0], []).append(((key[1],), code))
                continue
            wanted = component_forms[shape]
            wanted = wanted if len(key) == 3 else (wanted[0], wanted[2])
            liga = tuple(shapes[k][form] for k, form in zip(key, wanted))
            table = ligas_3 if len(key) == 3 else ligas_2
            table.setdefault(liga[0], []).append((liga[1:], code))
    return forms, ligas_2, ligas_3, ligas_mark


def hex_code(code):
    """A code point literal; long ones are grouped as clippy wants."""
    return "0x%X" % code if code <= 0xFFFFF else "0x%04X_%04X" % (code >> 16, code & 0xFFFF)


def wrap(items):
    """Array items packed into lines of at most 100 columns."""
    lines, cur = [], "   "
    for item in items:
        piece = " " + item + ","
        if len(cur) + len(piece) > 100:
            lines.append(cur)
            cur = "   "
        cur += piece
    lines.append(cur)
    return lines


def static(doc, declaration):
    """Doc comment, rustfmt opt-out (the packed layout keeps the file
    compact) and declaration line of a generated table."""
    return ["", *doc, "#[rustfmt::skip]", declaration]


def table(doc, declaration, items):
    """A generated static: doc comment, declaration and packed items."""
    return static(doc, declaration) + wrap(items) + ["];"]


def ligature_table(name, doc, ligatures, arity):
    out = static(doc, "pub(crate) static %s: [(u16, Ligatures<%d>); %d] = [" % (
        name, arity, len(ligatures)))
    for first in sorted(ligatures):
        entries = []
        for components, ligature in ligatures[first]:
            comps = ", ".join("0x%04X" % c for c in components)
            entries.append("([%s], 0x%04X)" % (comps, ligature))
        out.append("    (0x%04X, &[" % first)
        out += ["    " + line for line in wrap(entries)]
        out.append("    ]),")
    out.append("];")
    return out


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    ucd = pathlib.Path(sys.argv[1])
    entries = unicode_data(ucd)
    categories = general_categories(entries)
    category_runs = runs(categories)
    bidi_runs = runs(bidi_classes(ucd))
    joining = joining_types(ucd, categories)
    pairs = brackets(ucd)
    mirrored = mirrors(ucd)
    forms, ligas_2, ligas_3, ligas_mark = presentation_forms(ucd)
    gc_index = {name: i for i, name in enumerate(GENERAL_CATEGORIES)}
    bidi_index = {name: i for i, name in enumerate(BIDI_CLASSES)}
    out = [
        "//! Unicode 17.0.0 tables generated by `tools/gen_ucd_tables.py` from the",
        "//! Unicode Character Database. Do not edit by hand.",
    ]
    out += table(
        ["/// General category runs: `(first code point, category code)`; each run",
         "/// extends to the next entry. Codes index `GeneralCategory::ALL`."],
        "pub(crate) static GENERAL_CATEGORIES: [(u32, u8); %d] = [" % len(category_runs),
        ("(%s, %d)" % (hex_code(c), gc_index[v]) for c, v in category_runs))
    out += table(
        ["/// Bidi class runs: `(first code point, class code)`; each run extends to",
         "/// the next entry. Codes index `BidiClass::ALL`."],
        "pub(crate) static BIDI_CLASSES: [(u32, u8); %d] = [" % len(bidi_runs),
        ("(%s, %d)" % (hex_code(c), bidi_index[v]) for c, v in bidi_runs))
    out += table(
        ["/// Paired brackets `(bracket, pair, is_opening)`, sorted by bracket."],
        "pub(crate) static BRACKETS: [(u32, u32, bool); %d] = [" % len(pairs),
        ("(0x%04X, 0x%04X, %s)" % (a, b, "true" if o else "false") for a, b, o in pairs))
    out += table(
        ["/// Bidi mirroring glyphs `(character, mirror)`, sorted by character."],
        "pub(crate) static MIRRORS: [(u32, u32); %d] = [" % len(mirrored),
        ("(0x%04X, 0x%04X)" % m for m in mirrored))
    out += table(
        ["/// Joining type runs `(first, last, code)` for characters that are not",
         "/// non-joining; codes index `JoiningType::ALL`."],
        "pub(crate) static JOINING_TYPES: [(u32, u32, u8); %d] = [" % len(joining),
        ("(%s, %s, %d)" % (hex_code(a), hex_code(b), k) for a, b, k in joining))
    out += table(
        ["/// Arabic presentation forms `(character, [initial, medial, final,",
         "/// isolated])` (0 when the form does not exist)."],
        "pub(crate) static ARABIC_FORMS: [(u16, [u16; 4]); %d] = [" % len(forms),
        ("(0x%04X, [%s])" % (c, ", ".join("0x%04X" % f for f in row)) for c, row in forms))
    out += [
        "",
        "/// The ligatures starting with one component: `(other components,",
        "/// ligature)`, tried in order.",
        "pub(crate) type Ligatures<const N: usize> = &'static [([u16; N], u16)];",
    ]
    out += ligature_table(
        "ARABIC_LIGATURES_3",
        ["/// Three-component presentation-form ligatures, by first component."],
        ligas_3, 2)
    out += ligature_table(
        "ARABIC_LIGATURES",
        ["/// Two-component presentation-form ligatures, by first component."],
        ligas_2, 1)
    out += ligature_table(
        "ARABIC_MARK_LIGATURES",
        ["/// Mark ligatures (shadda with a vowel mark), by first mark."],
        ligas_mark, 1)
    OUT.write_text("\n".join(out) + "\n")
    print(f"{len(category_runs)} categories, {len(bidi_runs)} bidi runs, {len(pairs)} brackets, "
          f"{len(mirrored)} mirrors, {len(joining)} joining runs, {len(forms)} forms -> "
          f"{os.path.relpath(OUT)}")


if __name__ == "__main__":
    main()
