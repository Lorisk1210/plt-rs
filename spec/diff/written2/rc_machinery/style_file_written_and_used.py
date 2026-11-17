# Write .mplstyle files next to the program, use them by path, combine with a dict and a builtin style, and read back rcParams.
import os
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

os.makedirs("styles", exist_ok=True)
with open("styles/house.mplstyle", "w") as f:
    f.write("# house style\naxes.facecolor: f4f4f4\naxes.grid: True\ngrid.linestyle: --\n"
            "lines.linewidth: 2.5\naxes.prop_cycle: cycler('color', ['1b9e77', 'd95f02', '7570b3'])\n"
            "font.size: 9  # trailing comment\nfigure.figsize: 4, 3\n")
with open("styles/bad.mplstyle", "w") as f:
    f.write("axes.facecolor f4f4f4\nnot.a.param: 1\nlines.linewidth: wide\n")
for spec in ["styles/house.mplstyle", ["ggplot", "styles/house.mplstyle"], ["styles/house.mplstyle", {"lines.linewidth": 1}]]:
    with plt.style.context(spec):
        fig, ax = plt.subplots()
        ln, = ax.plot([0, 1], [0, 1])
        print(plt.rcParams["axes.facecolor"], plt.rcParams["lines.linewidth"], plt.rcParams["font.size"],
              plt.rcParams["figure.figsize"], ln.get_color(), ax.xaxis._major_tick_kw.get("gridOn"))
        fig.savefig(f"styled_{len(str(spec))}.png")
        plt.close(fig)
import warnings
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    with plt.style.context("styles/bad.mplstyle"):
        print(plt.rcParams["lines.linewidth"], plt.rcParams["axes.facecolor"])
for w in caught:
    print(type(w.message).__name__, str(w.message).splitlines()[0][:80])
rc = matplotlib.rc_params_from_file("styles/house.mplstyle", use_default_template=False)
print(sorted(rc.keys()), type(rc).__name__)
try:
    plt.style.use("styles/missing.mplstyle")
except OSError as e:
    print("OSError", str(e)[:60])
