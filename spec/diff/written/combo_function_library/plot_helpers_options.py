# A small plotting helper library (line_with_band, labeled_bars, finish) called with different options.
import numpy as np
import matplotlib.pyplot as plt


def line_with_band(ax, x, y, err, *, color=None, label=None, alpha=0.25):
    line, = ax.plot(x, y, color=color, label=label)
    ax.fill_between(x, y - err, y + err, color=line.get_color(), alpha=alpha, lw=0)
    return line


def labeled_bars(ax, labels, values, *, fmt="{:.0f}", horizontal=False, highlight=None):
    draw = ax.barh if horizontal else ax.bar
    bars = draw(labels, values, color=["C3" if l == highlight else "C0" for l in labels])
    ax.bar_label(bars, fmt=fmt, padding=2)
    return bars


def finish(fig, title, *, legend_axes=None, path=None):
    fig.suptitle(title)
    if legend_axes is not None:
        legend_axes.legend(loc="best")
    fig.tight_layout()
    if path:
        fig.savefig(path, dpi=50)
    return fig


x = np.linspace(0, 10, 50)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
l1 = line_with_band(a, x, np.sin(x), 0.2, label="sin")
l2 = line_with_band(a, x, np.cos(x), 0.1 + 0.02 * x, color="purple", label="cos", alpha=0.15)
bars = labeled_bars(b, ["a", "b", "c", "d"], [3.2, 5.1, 2.7, 4.4], fmt="{:.1f}", highlight="b")
finish(fig, "helpers", legend_axes=a, path="helpers.png")
fig2, ax2 = plt.subplots(figsize=(5, 3))
labeled_bars(ax2, ["x", "y"], [10, 20], horizontal=True)
finish(fig2, "horizontal", path="horizontal.png")
print(l1.get_color(), l2.get_color(), [b.get_facecolor() for b in bars][:2])
print([round(c.get_alpha(), 2) for c in a.collections])
