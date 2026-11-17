# Write a matplotlibrc file, load it with rc_params_from_file and rc_file, check comments, quoting and bad lines.
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

lines = [
    "## a comment line",
    "lines.linewidth : 3.5",
    "axes.titlecolor: '#336699'   # quoted hex",
    "font.family: serif",
    "font.serif: DejaVu Serif, Times New Roman",
    "axes.prop_cycle: cycler(color=['k', 'r'])",
    "figure.figsize: 5.0, 3.0",
    "savefig.bbox: tight",
    "text.color: 0.25",
    "axes.formatter.limits: -3, 3",
    "lines.dash_capstyle: round",
    "not_a_key: 5",
    "grid.alpha 0.5",
    "xtick.direction: sideways",
]
with open("matplotlibrc", "w") as f:
    f.write("\n".join(lines) + "\n")
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    rc = matplotlib.rc_params_from_file("matplotlibrc", fail_on_error=False, use_default_template=False)
for w in caught:
    print(str(w.message).splitlines()[0][:90])
for k in sorted(rc):
    print(k, repr(rc[k]))
with warnings.catch_warnings(record=True):
    warnings.simplefilter("ignore")
    matplotlib.rc_file("matplotlibrc")
print(plt.rcParams["lines.linewidth"], plt.rcParams["savefig.bbox"], plt.rcParams["font.serif"][:2])
try:
    matplotlib.rc_params_from_file("matplotlibrc", fail_on_error=True, use_default_template=False)
except ValueError as e:
    print("ValueError", str(e)[:60])
print(matplotlib.matplotlib_fname().endswith("matplotlibrc"))
plt.rcdefaults()
