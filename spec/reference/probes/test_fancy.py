"""Fancy box, arrow and connection styles (``matplotlib.patches.BoxStyle``,
``ArrowStyle`` and ``ConnectionStyle``): parsing ``"name,key=value"``
style strings and the paths each style makes.

Area ``fancy``. Paths are ``{"vertices": [[x, y], ...], "codes": [...] |
null}``, with codes that only say "move, then lines" written as null (the
same drawing as no codes).

Ops
---
``patches.BoxStyle``         {style, rect: [x0, y0, width, height], mutation_size}
    -> path of ``BoxStyle(style)(x0, y0, width, height, mutation_size)``
``patches.ArrowStyle``       {style, path, mutation_size, linewidth, aspect}
    -> ``[[path, fillable], ...]`` from ``ArrowStyle(style)(path, mutation_size,
    linewidth, aspect)`` (a single path becomes a one-element list)
``patches.ConnectionStyle``  {style, a: [x, y], b: [x, y]}
    -> path of ``ConnectionStyle(style).connect(a, b)``
"""

from matplotlib.patches import ArrowStyle, BoxStyle, ConnectionStyle
from matplotlib.path import Path

from spec_support import ProbeSet

probes = ProbeSet("fancy")


def _path(path):
    codes = None if path.codes is None else [int(c) for c in path.codes]
    if codes is not None and codes == [Path.MOVETO] + [Path.LINETO] * (len(codes) - 1):
        codes = None
    return {"vertices": [[float(x), float(y)] for x, y in path.vertices], "codes": codes}


def _box(style, rect, mutation_size):
    return _path(BoxStyle(style)(*rect, mutation_size))


def _arrow(style, path, mutation_size, linewidth, aspect):
    source = Path(path["vertices"], path["codes"])
    paths, fillable = ArrowStyle(style)(source, mutation_size, linewidth, aspect)
    if isinstance(paths, Path):
        return [[_path(paths), bool(fillable)]]
    return [[_path(p), bool(f)] for p, f in zip(paths, fillable)]


def _connection(style, a, b):
    return _path(ConnectionStyle(style).connect(tuple(a), tuple(b)))


BOXES = ["square", "circle", "ellipse", "larrow", "rarrow", "darrow", "round", "round4",
         "sawtooth", "roundtooth", "square,pad=0", "circle,pad=0.5", "round,pad=0.5",
         "round,rounding_size=0.2", "round,pad=0.1,rounding_size=1", "round4,pad=0.2",
         "round4,rounding_size=0.5", "sawtooth,tooth_size=0.5", "roundtooth,pad=0.2,tooth_size=1",
         "larrow,pad=0.6", "darrow,pad=0", "Round", " round , pad = 0.4 ", "bogus",
         "round,bogus=1", "round,pad=x", "round,pad"]
RECTS = [[0.0, 0.0, 10.0, 5.0], [1.0, 2.0, 0.0, 0.0], [0.0, 0.0, 3.0, 30.0], [-5.0, 4.0, 7.5, 2.5]]


def test_boxes():
    for style in BOXES:
        for rect in RECTS:
            for mutation_size in [1.0, 10.0]:
                probes.probe("patches.BoxStyle", _box, style=style, rect=rect,
                             mutation_size=mutation_size)


ARROWS = ["-", "<-", "->", "<->", "<|-", "-|>", "<|-|>", "]-", "-[", "]-[", "|-|", "]->",
          "<-[", "simple", "fancy", "wedge", "->,head_length=0.8,head_width=0.4",
          "<->,head_length=0.2,head_width=0.6", "-|>,head_length=1,head_width=0.2",
          "simple,head_length=0.8,head_width=0.8,tail_width=0.4",
          "fancy,head_length=0.6,head_width=0.6,tail_width=0.6",
          "wedge,tail_width=0.6,shrink_factor=0.2", "-[,widthB=2,lengthB=0.5,angleB=30",
          "]-[,widthA=1.5,widthB=0.5", "|-|,widthA=2,angleA=20,angleB=-20",
          "]->,widthA=1,lengthA=0.5", "bogus"]
LINE = {"vertices": [[0.0, 0.0], [100.0, 50.0]], "codes": [1, 2]}
QUAD = {"vertices": [[0.0, 0.0], [50.0, 80.0], [100.0, 0.0]], "codes": [1, 3, 3]}
BENT = {"vertices": [[0.0, 0.0], [0.0, 60.0], [80.0, 60.0]], "codes": [1, 2, 2]}


def test_arrows():
    for style in ARROWS:
        for path in [LINE, QUAD, BENT]:
            for mutation_size, linewidth, aspect in [(10.0, 1.0, 1.0), (20.0, 3.0, 1.0),
                                                     (10.0, 1.0, 2.0)]:
                probes.probe("patches.ArrowStyle", _arrow, style=style, path=path,
                             mutation_size=mutation_size, linewidth=linewidth, aspect=aspect)


CONNECTIONS = ["arc3", "arc3,rad=0.3", "arc3,rad=-0.5", "angle3", "angle3,angleA=0,angleB=90",
               "angle3,angleA=45,angleB=45", "angle", "angle,angleA=0,angleB=90,rad=10",
               "angle,angleA=-90,angleB=180,rad=0", "arc", "arc,angleA=0,angleB=0,armA=30,armB=30",
               "arc,angleA=90,angleB=-90,armA=0,armB=40,rad=5", "bar", "bar,fraction=0.5",
               "bar,armA=10,armB=20,fraction=0.1", "bar,angle=45,fraction=-0.2", "bogus"]
PAIRS = [([0.0, 0.0], [100.0, 50.0]), ([0.0, 0.0], [0.0, 100.0]), ([10.0, 10.0], [10.0, 10.0]),
         ([0.0, 0.0], [-50.0, 30.0])]


def test_connections():
    for style in CONNECTIONS:
        for a, b in PAIRS:
            probes.probe("patches.ConnectionStyle", _connection, style=style, a=a, b=b)
