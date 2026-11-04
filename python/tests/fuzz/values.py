"""
Random arguments for the fuzzer's programs, as Python source.

Values are source text, so that a program the fuzzer ran can be written
out and run again as it is. Each kind of value mixes what matplotlib
accepts with the borderline (empty, zero, NaN, infinite, huge, negative)
and the absurd (the wrong type, nested, ragged); a parameter's name
chooses the kind (``color`` a colour, ``x`` an array...), and one value in
six is of any kind at all. How often the borderline and absurd come
is the program's *wildness*: the lists below start with the ordinary
values, `ORDINARY` long.
"""

#: Numbers, from the ordinary to the absurd.
NUMBERS = [
    "0", "1", "-1", "2", "3", "0.5", "-0.5", "10", "100", "1e-9", "1e-300",
    "1e300", "-1e300", "1e15", "2**31", "2**63", "-2**63", "float('nan')",
    "float('inf')", "-float('inf')", "np.float32(1.5)", "np.int64(7)", "True",
]

#: Arrays of numbers: shapes and contents matplotlib must cope with.
ARRAYS = [
    "[]", "[1]", "[1, 2, 3]", "[0, 0, 0]", "[3, 1, 2, 5]", "np.arange(10)",
    "np.linspace(-5, 5, 50)", "np.sin(np.linspace(0, 10, 200))",
    "[1, float('nan'), 3]", "[float('nan')] * 4", "[1, float('inf'), -float('inf')]",
    "[1e300, -1e300, 1]", "[1e-300, 2e-300]", "np.zeros(0)", "np.zeros((0, 2))",
    "np.ones((3, 4))", "np.arange(12).reshape(3, 4)", "np.eye(5)",
    "np.random.default_rng(1).normal(size=300)",
    "np.random.default_rng(2).random((20, 30))",
    "np.ma.masked_invalid([1, float('nan'), 3, 4])",
    "np.ma.masked_all(3)", "[[1, 2], [3]]", "[[[1]]]", "[1, 'a', None]",
    "['a', 'b', 'c']", "[True, False, True]", "np.array([1, 2, 3], dtype=np.int8)",
    "np.array([2**62, -2**62])", "np.datetime64('2020-01-01') + np.arange(3)",
    "range(5)", "(i * i for i in range(4))", "np.full(4, 7.0)",
]

#: Two-dimensional fields, for images, meshes and contours.
FIELDS = [
    "np.arange(12.0).reshape(3, 4)", "np.random.default_rng(3).random((15, 10))",
    "np.zeros((5, 5))", "np.full((4, 4), float('nan'))", "[[1, 2], [3, 4]]",
    "np.ones((1, 1))", "np.zeros((0, 0))", "np.eye(3) * 1e300",
    "np.ma.masked_greater(np.arange(16.0).reshape(4, 4), 9)",
    "np.random.default_rng(4).random((6, 7, 3))", "np.random.default_rng(5).random((6, 7, 4))",
    "np.array([[1, float('inf')], [-float('inf'), 0]])", "[[1, 2, 3]]", "[1, 2, 3]",
    "np.arange(24).reshape(2, 3, 4)", "np.array([[True, False], [False, True]])",
]

#: Colours, valid and not.
COLORS = [
    "'red'", "'C1'", "'#00ff00'", "'#0f08'", "'none'", "'0.5'", "(1, 0, 0)",
    "(0, 0, 1, 0.5)", "'tab:blue'", "'xkcd:sky blue'", "[0.1, 0.2, 0.3]",
    "'notacolor'", "(2, 0, 0)", "'#ggg'", "''", "[]", "(float('nan'), 0, 0)",
    "['r', 'g', 'b']", "np.random.default_rng(6).random((4, 3))", "None",
]

#: Strings: labels, mathtext, control characters, the huge.
STRINGS = [
    "''", "'x'", "'label'", "'$x^2$'", "'$\\\\frac{1}{2}$'", "'$\\\\frac{'", "'$$'", "'\\\\'",
    "'a' * 5000", "'\\x00'", "'line\\nbreak\\n\\n'", "'\\u03b1\\u2603\\U0001F600'",
    "'_nolegend_'", "'%d %s'", "'{x}'", "'$\\\\mathrm{a}_{b}^{c}$'",
    "'$\\\\sqrt[3]{x}$'", "'\\t'", "' '", "'$\\\\undefinedmacro$'",
]

#: Keyword values of properties that take names.
NAMED = {
    "linestyle": ["'-'", "'--'", "':'", "'-.'", "'none'", "(0, (5, 2))", "(0, ())", "'dotted'",
                  "(0, (0, 0))", "(0, (-1, 2))", "'wiggly'", "(float('nan'), (1, 1))"],
    "marker": ["'o'", "'s'", "'^'", "'.'", "'$\\\\alpha$'", "'none'", "None", "'x'", "8",
               "(5, 1, 0)", "'not-a-marker'", "[(0, 0), (1, 1), (0, 1)]", "'$'"],
    "cmap": ["'viridis'", "'gray'", "'jet_r'", "'twilight'", "'notacmap'", "None"],
    "loc": ["'best'", "'upper right'", "'center'", "0", "10", "(0.5, 0.5)", "'nowhere'", "-1"],
    "ha": ["'left'", "'center'", "'right'", "'middle'"],
    "va": ["'top'", "'center'", "'baseline'", "'center_baseline'", "'nowhere'"],
    "norm": ["'log'", "'linear'", "colors.LogNorm()", "colors.Normalize(0, 1)",
             "colors.Normalize(1, 0)", "colors.SymLogNorm(1)", "colors.BoundaryNorm([0, 1, 2], 3)",
             "colors.TwoSlopeNorm(0)", "colors.PowerNorm(0.5)", "'nonorm'"],
    "scale": ["'linear'", "'log'", "'symlog'", "'logit'", "'asinh'", "'function'", "'nope'"],
    "transform": ["ax.transData", "ax.transAxes", "fig.transFigure", "None",
                  "transforms.Affine2D().rotate_deg(30) + ax.transData",
                  "transforms.Affine2D().scale(0)", "transforms.IdentityTransform()"],
    "interpolation": ["'nearest'", "'bilinear'", "'antialiased'", "'lanczos'", "'none'", "'bogus'"],
    "aspect": ["'equal'", "'auto'", "1", "0", "-1", "float('nan')", "1e300", "'nope'"],
    "hatch": ["'/'", "'xx'", "'*O.'", "''", "'?'", "'/' * 50"],
    "drawstyle": ["'default'", "'steps-mid'", "'steps-pre'", "'zigzag'"],
    "capstyle": ["'butt'", "'round'", "'projecting'", "'flat'"],
    "joinstyle": ["'miter'", "'round'", "'bevel'", "'wobbly'"],
    "fontsize": ["8", "'large'", "0", "-5", "1e5", "'huge'", "float('nan')"],
    "fontweight": ["'bold'", "100", "1000", "'feather'"],
    "fontfamily": ["'serif'", "'monospace'", "'DejaVu Sans'", "'No Such Font'", "['cursive', 'serif']"],
    "orientation": ["'vertical'", "'horizontal'", "'diagonal'"],
    "bbox_inches": ["None", "'tight'", "transforms.Bbox([[0, 0], [1, 1]])",
                    "transforms.Bbox([[0, 0], [0, 0]])", "transforms.Bbox([[1, 1], [0, 0]])"],
    "align": ["'center'", "'edge'", "'left'"],
    "snap": ["True", "False", "None"],
    "picker": ["True", "5", "False", "None", "-1", "lambda artist, event: (True, {})"],
}

#: Values of the parameters that, given anything else true, wait for the
#: user's clicks (as matplotlib's do): they get these only.
WAITING = {
    "manual": ["False", "None", "[]", "[(1, 1), (2, 2)]"],
    "block": ["False", "None"],
}

#: Which kind of value a parameter takes, from its name.
KINDS = [
    (("color", "colors", "fc", "ec", "mfc", "mec", "c"), "color"),
    (("label", "title", "text", "s", "fmt", "name", "xlabel", "ylabel", "t"), "string"),
    (("x", "y", "z", "data", "values", "heights", "height", "width", "widths", "xerr", "yerr",
      "bins", "weights", "levels", "positions", "dataset", "xy", "xytext", "ticks", "locs",
      "offsets", "sizes", "verts", "u", "v", "args", "left", "bottom"), "array"),
    (("C", "X", "Y", "Z", "A", "im", "arr", "image"), "field"),
]

#: How many values at the start of each list are ordinary ones.
ORDINARY = {"NUMBERS": 10, "ARRAYS": 8, "FIELDS": 3, "COLORS": 11, "STRINGS": 7}

#: Words in a parameter's name that say it takes a number.
NUMBER_WORDS = ("width", "size", "alpha", "dpi", "pad", "zorder", "angle", "rotation", "scale",
                "radius", "lim", "min", "max", "num", "every", "spacing", "length", "frac")

#: Objects of every other kind.
OTHERS = ["None", "True", "False", "object()", "{}", "{'a': 1}", "()", "lambda *a, **k: None",
          "plt", "...", "b''", "slice(1)"]


def choose(rng, items):
    """One of *items*."""
    return items[rng.randrange(len(items))]


class Values:
    """Random values of a program, drawn from *rng*: borderline and
    absurd ones with probability *wild*, else ordinary ones."""

    def __init__(self, rng, wild):
        self.rng = rng
        self.wild = wild

    def _pick(self, name):
        items = globals()[name]
        if self.rng.random() >= self.wild:
            items = items[:ORDINARY[name]]
        return choose(self.rng, items)

    def number(self):
        """A number expression."""
        return self._pick("NUMBERS")

    def array(self):
        """An array expression."""
        return self._pick("ARRAYS")

    def field(self):
        """A two-dimensional field expression."""
        return self._pick("FIELDS")

    def color(self):
        """A colour expression."""
        return self._pick("COLORS")

    def string(self):
        """A string expression."""
        return self._pick("STRINGS")

    def named(self, key):
        """A value of the property *key* of `NAMED`."""
        return choose(self.rng, NAMED[key])

    def any_value(self):
        """A value of any kind, the absurd included."""
        rng = self.rng
        kind = rng.randrange(9)
        if kind == 5:
            return self.named(choose(rng, list(NAMED)))
        if kind == 6:
            return choose(rng, OTHERS)
        if kind == 7:
            return "(" + ", ".join(self.number() for _ in range(rng.randrange(1, 5))) + ")"
        if kind == 8:
            return "[" + ", ".join(self.any_value() for _ in range(rng.randrange(0, 3))) + "]"
        return [self.number, self.array, self.field, self.color, self.string][kind]()

    def for_parameter(self, name):
        """A value for a parameter called *name*: of the kind its name
        says, or (now and then, and for names saying nothing) of any
        kind."""
        rng = self.rng
        bare = name.lower().lstrip("_")
        if bare in WAITING:
            return choose(rng, WAITING[bare])
        if rng.random() < self.wild / 3:
            return self.any_value()
        for key in NAMED:
            if bare == key or bare.endswith(key):
                return self.named(key)
        for names, kind in KINDS:
            if name in names or bare in names:
                return {"color": self.color, "string": self.string, "array": self.array,
                        "field": self.field}[kind]()
        if any(word in bare for word in NUMBER_WORDS):
            return self.number()
        if bare.startswith(("is_", "use", "show", "draw", "visible", "auto", "clip", "animated")):
            return choose(rng, ["True", "False", "None", "0", "'yes'"])
        return self.any_value()
