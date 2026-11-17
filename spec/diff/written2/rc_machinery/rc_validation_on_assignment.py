# Assign rcParams with strings, numbers and wrong types and print the stored, validated value or the error.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

trials = [
    ("lines.linewidth", "2"), ("lines.linewidth", 2), ("lines.linewidth", "two"),
    ("axes.grid", "on"), ("axes.grid", 0), ("axes.grid", "sometimes"),
    ("figure.figsize", "6, 4"), ("figure.figsize", [6, 4, 2]), ("figure.figsize", (3, 3)),
    ("lines.linestyle", "--"), ("lines.linestyle", (0, (5, 2))), ("lines.linestyle", "wavy"),
    ("axes.titlelocation", "right"), ("axes.titlelocation", "middle"),
    ("legend.loc", "best"), ("legend.loc", 3), ("legend.loc", "corner"),
    ("image.interpolation", "nearest"), ("image.interpolation", "smooth"),
    ("savefig.format", "PNG"), ("savefig.dpi", "figure"), ("savefig.dpi", "-5"),
    ("font.size", "large"), ("font.size", "12"),
    ("axes.edgecolor", (1, 0, 0)), ("axes.edgecolor", "#12345"),
    ("markers.fillstyle", "top"), ("markers.fillstyle", "half"),
    ("hatch.linewidth", "1.5"), ("contour.negative_linestyle", "dotted"),
    ("date.converter", "concise"), ("date.converter", "short"),
    ("scatter.marker", "D"), ("scatter.marker", 12),
    ("ytick.alignment", "center_baseline"), ("ytick.alignment", "middle"),
]
for key, value in trials:
    try:
        with plt.rc_context({key: value}):
            print(key, repr(value), "->", repr(plt.rcParams[key]))
    except (ValueError, TypeError) as e:
        print(key, repr(value), type(e).__name__, str(e).splitlines()[0][:70])
plt.rcParams.update({"lines.linewidth": "4.5"})
print(repr(plt.rcParams["lines.linewidth"]))
print(repr(plt.rcParams.get("no.such.key", "missing")))
