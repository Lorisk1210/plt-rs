# Call rcsetup validators directly with good and bad values and print what they return or the error class.
from matplotlib import rcsetup

cases = [
    ("validate_bool", ["yes", "off", 1, "t", "maybe", None]),
    ("validate_float", ["1.5", 2, "x", None]),
    ("validate_float_or_None", [None, "None", "3"]),
    ("validate_int", ["4", 4.0, "4.5"]),
    ("validate_color", ["red", "#abc", "0.5", "C3", "none", "notacolor", "(1, 0, 0)"]),
    ("validate_color_or_auto", ["auto", "blue"]),
    ("validate_fontsize", ["large", "12", 12.5, "huge"]),
    ("validate_fontweight", ["bold", "700", 700, "fat"]),
    ("validate_aspect", ["auto", "equal", "2", "wide"]),
    ("validate_axisbelow", ["line", True, "false", "under"]),
    ("validate_dpi", ["figure", "72", "high"]),
    ("validate_stringlist", ["a, b,c", ["x", "y"], ""]),
    ("validate_floatlist", ["1, 2.5", [3, 4]]),
    ("validate_hatch", ["//", "x+", "q"]),
    ("validate_sketch", ["None", "1, 2, 3", (1, 2, 3)]),
    ("validate_fonttype", ["3", "42", "5"]),
    ("validate_bbox", ["tight", "standard", None]),
]
for name, values in cases:
    fn = getattr(rcsetup, name)
    out = []
    for v in values:
        try:
            out.append(repr(fn(v)))
        except (ValueError, TypeError) as e:
            out.append(type(e).__name__)
    print(name, out)
print(rcsetup.validate_cycler("cycler('color', 'rgb') * cycler('linestyle', ['-', '--'])"))
for bad in ["cycler('colour', 'rgb')", "cycler(color='rg') + cycler(lw=[1])", "__import__('os')"]:
    try:
        rcsetup.validate_cycler(bad)
    except ValueError as e:
        print("ValueError", str(e)[:60])
