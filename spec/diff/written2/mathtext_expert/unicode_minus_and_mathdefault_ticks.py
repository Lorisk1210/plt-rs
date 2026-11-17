# axes.unicode_minus and axes.formatter.use_mathtext on tick labels of linear, log and symlog axes; label strings and widths.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for unicode_minus in (True, False):
    for use_math in (False, True):
        with plt.rc_context({"axes.unicode_minus": unicode_minus, "axes.formatter.use_mathtext": use_math}):
            fig, axs = plt.subplots(1, 3, figsize=(8, 2.5))
            axs[0].plot([-3e-5, 2e-5], [-1, 1])
            axs[1].set_yscale("log")
            axs[1].plot([1, 2], [1e-3, 1e4])
            axs[2].set_xscale("symlog")
            axs[2].plot([-1000, 1000], [0, 1])
            fig.canvas.draw()
            print(unicode_minus, use_math, [t.get_text() for t in axs[0].get_xticklabels()][:3],
                  axs[0].xaxis.get_offset_text().get_text())
            print([t.get_text() for t in axs[1].get_yticklabels()][:3], [t.get_text() for t in axs[2].get_xticklabels()][:3])
            print(round(axs[0].get_xticklabels()[0].get_window_extent().width, 2))
            fig.savefig(f"minus_{int(unicode_minus)}{int(use_math)}.png")
            plt.close(fig)
fig, ax = plt.subplots()
ax.plot([0, 1], [0, 1])
ax.set_xlabel(r"$\mathdefault{default}\ x$")
ax.set_ylabel(r"$\mathregular{regular}\ y$")
fig.canvas.draw()
print(round(ax.xaxis.label.get_window_extent().width, 2), round(ax.yaxis.label.get_window_extent().height, 2))
fig.savefig("labels.png")
ax.set_title(r"$-1$ and -1")
fig.canvas.draw()
print(repr(ax.title.get_text()), round(ax.title.get_window_extent().width, 2))
