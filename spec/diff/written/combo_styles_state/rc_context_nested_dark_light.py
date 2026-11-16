# Nested rc_context: a dark-themed figure with a light inset panel and a colour bar, rc values printed at each level.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:50j, 0:1:50j]
with plt.style.context("dark_background"):
    with mpl.rc_context({"axes.titlesize": 14, "lines.linewidth": 2}):
        fig, ax = plt.subplots(figsize=(7, 5))
        im = ax.imshow(np.sin(6 * x) * y, cmap="magma")
        fig.colorbar(im, ax=ax)
        print("outer", mpl.rcParams["axes.facecolor"], mpl.rcParams["axes.titlesize"])
        with mpl.rc_context({"axes.facecolor": "white", "axes.edgecolor": "black", "text.color": "black",
                             "xtick.color": "black", "ytick.color": "black"}):
            ins = ax.inset_axes([0.6, 0.6, 0.35, 0.35])
            ins.plot([0, 1, 2], [2, 0, 1])
            ins.set_title("light inset", fontsize=8)
            print("inner", mpl.rcParams["axes.facecolor"], ins.get_facecolor())
        ax.set_title("dark main")
        fig.savefig("nested.png", dpi=50)
print("after", mpl.rcParams["axes.facecolor"], mpl.rcParams["lines.linewidth"])
