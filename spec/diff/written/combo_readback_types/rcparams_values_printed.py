# Print rcParams values and their types after updates, as users debug styles, then draw with them.
import matplotlib as mpl
import matplotlib.pyplot as plt

mpl.rcParams.update({"lines.linewidth": "3", "axes.titlesize": "large", "figure.figsize": "5, 3", "axes.grid": "True",
                     "lines.dashed_pattern": [4, 2], "axes.prop_cycle": "cycler('color', ['r', 'g'])"})
for key in ["lines.linewidth", "axes.titlesize", "figure.figsize", "axes.grid", "lines.dashed_pattern", "axes.prop_cycle",
            "font.family", "image.cmap", "legend.loc", "savefig.bbox"]:
    v = mpl.rcParams[key]
    print(key, type(v).__name__, repr(v))
fig, ax = plt.subplots()
ax.plot([0, 1], ls="--")
ax.plot([1, 0])
ax.set_title("rc")
print([l.get_color() for l in ax.lines], ax.lines[0].get_linewidth())
fig.savefig("rc.png", dpi=50)
