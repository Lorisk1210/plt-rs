"""Build small fonts with Apple Advanced Typography tables and record
matplotlib's layouts of a few strings with them (libraqm + HarfBuzz),
which shapes with `morx`, `kerx`, the Apple `kern` and `trak` in place of
the OpenType tables:

* `aat-morx.ttf`: a `morx` chain with each subtable type (a ligature of
  "fi" and "fl", a noncontextual "g" that `ss01` turns on, a contextual
  "x" before "y", a rearranged "rs" and a glyph inserted after "k"),
  with the `feat` table and chain features `liga` and `ss01` switch;
* `aat-kern.ttf`: an Apple `kern` table with formats 0, 2 and 3, a
  variation subtable HarfBuzz skips and a cross-stream state machine
  (format 1);
* `aat-kerx.ttf`: a `kerx` table with formats 0, 2 and 6;
* `aat-trak.ttf`: a `trak` table (with the `STAT` table HarfBuzz needs
  to apply it) tracking every grapheme, marks included in theirs.

Every glyph is a box with its own advance. Run from the repository root
with the project venv:

    MPL_IGNORE_SYSTEM_FONTS=1 MPLCONFIGDIR=$(mktemp -d) \\
        .venv/bin/python -I crates/pltrs-font/tests/fixtures/gen_aat_fonts.py

Writes the fonts and `aat.json` next to this script.
"""

import json
import pathlib
import struct

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables.DefaultTable import DefaultTable
from matplotlib.ft2font import FT2Font, LoadFlags

HERE = pathlib.Path(__file__).resolve().parent
NO_HINTING = LoadFlags.NO_HINTING
SIZES = [(12.0, 72.0), (10.0, 100.0)]
LETTERS = "abcdefgiklmnorstxyAELRTVWY"
MARK = 0x0301


def u16(*values):
    return b"".join(struct.pack(">H", v & 0xFFFF) for v in values)


def u32(*values):
    return b"".join(struct.pack(">I", v & 0xFFFFFFFF) for v in values)


def box(width):
    pen = TTGlyphPen(None)
    pen.moveTo((40, 0))
    pen.lineTo((40, 500))
    pen.lineTo((max(width, 100) - 40, 500))
    pen.lineTo((max(width, 100) - 40, 0))
    pen.closePath()
    return pen.glyph()


def build(path, extra_glyphs, tables):
    """A font of LETTERS, space, a combining acute and `extra_glyphs`
    (unmapped), boxes of distinct advances, with the raw `tables`."""
    names = [".notdef", "space"] + ["u%04X" % ord(c) for c in LETTERS] + ["acute"] + extra_glyphs
    cmap = {0x20: "space", MARK: "acute"}
    cmap.update({ord(c): "u%04X" % ord(c) for c in LETTERS})
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap(cmap)
    glyphs, metrics = {}, {}
    for index, name in enumerate(names):
        width = 0 if name == "acute" else 300 + 23 * index
        glyphs[name] = box(width)
        metrics[name] = (width, 40)
    glyphs["space"] = TTGlyphPen(None).glyph()
    metrics["space"] = (250, 0)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=900, descent=-300)
    fb.setupNameTable({"familyName": path.stem, "styleName": "Regular"})
    fb.setupOS2(sTypoAscender=900, sTypoDescender=-300, usWinAscent=900, usWinDescent=300)
    fb.setupPost()
    # Fixed dates, so that the fonts are the same from run to run.
    fb.updateHead(created=0, modified=0)
    font = fb.font
    font.recalcTimestamp = False
    gid = {name: font.getGlyphID(name) for name in names}
    for tag, make in tables.items():
        table = DefaultTable(tag)
        table.data = make(gid, len(names))
        font[tag] = table
    font.save(str(path))


def g(c):
    return "u%04X" % ord(c)


def lookup8(gid, mapping):
    """A lookup table of format 8 (a trimmed array) over `mapping`."""
    ids = {gid[k]: v for k, v in mapping.items()}
    first, last = min(ids), max(ids)
    return u16(8, first, last - first + 1, *[ids.get(i, 0) for i in range(first, last + 1)])


def state_table(gid, classes, rows, entries, extra_fields, blobs):
    """An extended state table: `classes` maps glyph names to classes
    (4 and up), `rows` lists the entry index for each class of each state,
    `entries` the (new state, flags, data words) entries; `extra_fields`
    32-bit header fields follow the table header and `blobs` (offsets
    filled into them, from the state table) follow everything."""
    n_classes = max(classes.values()) + 1
    header_len = 16 + 4 * len(extra_fields)
    class_table = lookup8(gid, classes)
    class_table += b"\0" * (len(class_table) % 4)
    states = b"".join(u16(*row) for row in rows)
    states += b"\0" * (len(states) % 4)
    entry_bytes = b"".join(u16(new, flags, *data) for new, flags, data in entries)
    entry_bytes += b"\0" * (len(entry_bytes) % 4)
    class_at = header_len
    states_at = class_at + len(class_table)
    entries_at = states_at + len(states)
    at = entries_at + len(entry_bytes)
    offsets = []
    for blob in blobs:
        offsets.append(at)
        at += len(blob) + len(blob) % 4
    fields = [offsets[i] if isinstance(f, int) and f < 0 else f for i, f in enumerate(extra_fields)]
    body = u32(n_classes, class_at, states_at, entries_at, *fields) + class_table + states + entry_bytes
    for blob in blobs:
        body += blob + b"\0" * (len(blob) % 4)
    return body


def subtable(kind, flags, body, coverage=0):
    return u32(12 + len(body), (coverage << 24) | kind, flags) + body


def morx_table(gid, num_glyphs):
    f, i, l = gid[g("f")], gid[g("i")], gid[g("l")]
    # Ligatures: f then i or l. Components sum to an index of the ligature list.
    components = [0] * num_glyphs
    components[i], components[l] = 1, 2
    ligature = state_table(
        gid, {g("f"): 4, g("i"): 5, g("l"): 6},
        [[0, 0, 0, 0, 1, 0, 0], [0, 0, 0, 0, 1, 0, 0], [0, 0, 0, 0, 1, 2, 3]],
        [(0, 0, [0]), (2, 0x8000, [0]), (0, 0xA000, [0]), (0, 0xA000, [2])],
        [-1, -1, -1],
        [u32(0, 0x80000000, 0, 0x80000000), u16(*components),
         u16(0, gid["f_i"], gid["f_l"])])
    # Noncontextual: g -> g.alt.
    noncontextual = lookup8(gid, {g("g"): gid["g.alt"]})
    # Contextual: an x followed by y becomes x.ctx.
    lists = u32(4) + lookup8(gid, {g("x"): gid["x.ctx"]})
    contextual = state_table(
        gid, {g("x"): 4, g("y"): 5},
        [[0, 0, 0, 0, 1, 0], [0, 0, 0, 0, 1, 0], [0, 0, 0, 0, 1, 2]],
        [(0, 0, [0xFFFF, 0xFFFF]), (2, 0x8000, [0xFFFF, 0xFFFF]), (0, 0, [0, 0xFFFF])],
        [-1], [lists])
    # Rearrangement: r ... s with verb 1 (Ax => xA).
    rearrangement = state_table(
        gid, {g("r"): 4, g("s"): 5},
        [[0, 0, 0, 0, 1, 0], [0, 0, 0, 0, 1, 0], [0, 0, 0, 0, 1, 2]],
        [(0, 0, []), (2, 0x8000, []), (0, 0x2001, [])],
        [], [])
    # Insertion: one glyph inserted after each k.
    insertion = state_table(
        gid, {g("k"): 4},
        [[0, 0, 0, 0, 1], [0, 0, 0, 0, 1]],
        [(0, 0, [0xFFFF, 0xFFFF]), (0, 1 << 5, [0, 0xFFFF])],
        [-1], [u16(gid["inserted"])])
    subtables = (subtable(2, 1, ligature) + subtable(4, 2, noncontextual)
                 + subtable(1, 4, contextual) + subtable(0, 8, rearrangement)
                 + subtable(5, 16, insertion))
    # Ligatures on by default (liga: type 1, settings 2 and 3); the
    # alternate g off (ss01: type 35, settings 2 and 3).
    features = (u16(1, 2) + u32(1, 0xFFFFFFFF) + u16(1, 3) + u32(0, 0xFFFFFFFE)
                + u16(35, 2) + u32(2, 0xFFFFFFFF) + u16(35, 3) + u32(0, 0xFFFFFFFD))
    chain = u32(0x1D, 16 + len(features) + len(subtables), 4, 5) + features + subtables
    return u16(2, 0) + u32(1) + chain


def feat_table(gid, num_glyphs):
    settings = 12 + 2 * 12
    return (u32(0x00010000) + u16(2, 0) + u32(0)
            + u16(1, 2) + u32(settings) + u16(0, 260)
            + u16(35, 2) + u32(settings + 8) + u16(0, 261)
            + u16(2, 262, 3, 263, 2, 264, 3, 265))


def kern_table(gid, num_glyphs):
    A, V, T, o = gid[g("A")], gid[g("V")], gid[g("T")], gid[g("o")]
    pairs = sorted([(A, V, -80), (T, o, -60)])
    def format0(pairs, coverage):
        body = u16(len(pairs), 12, 1, 0) + b"".join(u16(l, r) + struct.pack(">h", v) for l, r, v in pairs)
        return u32(8 + len(body)) + bytes([coverage, 0]) + u16(0) + body
    # Format 2: W, Y on the left; a, e on the right (b, c and d out of
    # bounds, which takes them out of the right-hand glyphs). Class values
    # are byte offsets that add up to a position in the value array.
    right_first = gid[g("a")]
    right_values = [0, 1, 1, 1, 2]
    left_at = 16
    right_at = left_at + 8
    array_at = right_at + 4 + 2 * len(right_values)
    left = u16(gid[g("W")], 2, array_at, array_at + 4)
    right = u16(right_first, len(right_values), *right_values)
    values = struct.pack(">hhhh", -40, -25, -70, -15)
    body2 = u16(4, left_at, right_at, array_at) + left + right + values
    format2 = u32(8 + len(body2)) + bytes([0, 2]) + u16(0) + body2
    # Format 3: classes for L and E (left), T and Y (right).
    count = num_glyphs
    left3 = [0] * count
    right3 = [0] * count
    left3[gid[g("L")]], left3[gid[g("E")]] = 1, 2
    right3[gid[g("T")]], right3[gid[g("Y")]] = 1, 2
    index3 = [0, 0, 0, 0, 1, 2, 0, 3, 0]
    body3 = (u16(count) + bytes([4, 3, 3, 0]) + struct.pack(">hhhh", 0, -90, -45, 30)
             + bytes(left3) + bytes(right3) + bytes(index3))
    format3 = u32(8 + len(body3)) + bytes([0, 3]) + u16(0) + body3
    # Format 1, cross-stream: m pushed, the next n raises it by 120 units.
    # The state table's header (classes, class table, states, entries and
    # the value table, 16 bits each) is followed by the class table.
    m, n = gid[g("m")], gid[g("n")]
    header = 10
    classes = u16(m, n - m + 1) + bytes([4] + [1] * (n - m - 1) + [5])
    classes += b"\0" * (len(classes) % 2)
    state_at = header + len(classes)
    rows = bytes([0, 0, 0, 0, 1, 0]) * 2 + bytes([0, 0, 0, 0, 1, 2])
    entries_at = state_at + len(rows)
    actions_at = entries_at + 12
    entries = u16(state_at, 0) + u16(state_at + 12, 0x8000) + u16(state_at, actions_at)
    machine = u16(6, header, state_at, entries_at, actions_at) + classes + rows + entries + struct.pack(">h", 121)
    format1 = u32(8 + len(machine)) + bytes([0x40, 1]) + u16(0) + machine
    variation = format0([(A, V, 500)], 0x20)
    tables = [format0(pairs, 0), variation, format2, format3, format1]
    return u32(0x00010000, len(tables)) + b"".join(tables)


def kerx_table(gid, num_glyphs):
    A, V, T, o = gid[g("A")], gid[g("V")], gid[g("T")], gid[g("o")]
    pairs = sorted([(A, V, -70), (T, o, -50)])
    body0 = u32(len(pairs), 12, 1, 0) + b"".join(u16(l, r) + struct.pack(">h", v) for l, r, v in pairs)
    format0 = u32(12 + len(body0), 0, 0) + body0
    # Format 2: classes are indices into the value array.
    left = lookup8(gid, {g("W"): 0, g("Y"): 2})
    right = lookup8(gid, {g("a"): 0, g("e"): 1})
    left_at = 12 + 16
    right_at = left_at + len(left)
    array_at = right_at + len(right)
    body2 = u32(2, left_at, right_at, array_at) + left + right + struct.pack(">hhhh", -30, -20, -60, -10)
    format2 = u32(12 + len(body2), 2, 0) + body2
    # Format 6: row and column indices into a short matrix.
    rows = lookup8(gid, {g("L"): 0, g("E"): 2})
    columns = lookup8(gid, {g("T"): 0, g("Y"): 1})
    rows_at = 12 + 24
    columns_at = rows_at + len(rows)
    array_at6 = columns_at + len(columns)
    body6 = (u32(0) + u16(2, 2) + u32(rows_at, columns_at, array_at6, 0) + rows + columns
             + struct.pack(">hhhh", -85, 25, -35, 15))
    format6 = u32(12 + len(body6), 6, 0) + body6
    return u16(2, 0) + u32(3) + format0 + format2 + format6


def trak_table(gid, num_glyphs):
    data = 12
    sizes_at = data + 8 + 2 * 8
    values_at = sizes_at + 3 * 4
    tracks = (struct.pack(">i", -65536) + u16(256, values_at)
              + struct.pack(">i", 0) + u16(257, values_at + 6))
    return (u32(0x00010000) + u16(0, data, 0, 0)
            + u16(2, 3) + u32(sizes_at) + tracks
            + struct.pack(">iii", 9 << 16, 12 << 16, 24 << 16)
            + struct.pack(">hhhhhh", -60, -40, -20, 20, 35, 70))


def stat_table(gid, num_glyphs):
    return u16(1, 1, 8, 0) + u32(0) + u16(0) + u32(0) + u16(2)


FONTS = {
    "morx": ({"morx": morx_table, "feat": feat_table},
             ["f_i", "f_l", "g.alt", "x.ctx", "inserted"],
             ["fi fl ffi", "gag", "xy xxy yx", "rs sr", "kok", "fig rsk xy"],
             [[], ["-liga"], ["ss01"], ["ss01[1:2]", "liga[0:1]"], ["-liga[4:6]"]]),
    "kern": ({"kern": kern_table}, [],
             ["AVATAR To", "Wa We Ya Ye", "LT LY ET EY", "mn mnm amn", "AV VA"],
             [[], ["-kern"], ["-kern[1:3]"]]),
    "kerx": ({"kerx": kerx_table}, [],
             ["AVATAR To", "Wa We Ya Ye", "LT LY ET EY"],
             [[], ["-kern"]]),
    "trak": ({"trak": trak_table, "STAT": stat_table}, [],
             ["track", "áb é", "AVATAR"],
             [[]]),
}


def layouts(path, strings, feature_sets):
    font = FT2Font(str(path))
    cases = []
    for size, dpi in SIZES:
        font.set_size(size, dpi)
        for text in strings:
            for features in feature_sets:
                items = [[int(item.glyph_index), round(item.x * 64), round(item.y * 64)]
                         for item in font._layout(text, NO_HINTING, features=features)]
                font.set_text(text, 0.0, flags=NO_HINTING, features=features)
                width, height = font.get_width_height()
                cases.append({"size": size, "dpi": dpi, "text": text, "features": features,
                              "glyphs": items,
                              "extents": [int(width), int(height), int(font.get_descent())]})
    return cases


def main():
    data = {}
    for name, (tables, extra, strings, feature_sets) in FONTS.items():
        path = HERE / f"aat-{name}.ttf"
        build(path, extra, tables)
        data[name] = layouts(path, strings, feature_sets)
    (HERE / "aat.json").write_text(json.dumps(data, ensure_ascii=True, separators=(",", ":")) + "\n")
    print("wrote", ", ".join(f"aat-{name}.ttf" for name in FONTS), "and", HERE / "aat.json")


if __name__ == "__main__":
    main()
