# 10^4 individual Rectangle patches versus one PatchCollection of the same rectangles; drawn output compared pixel for pixel.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.collections import PatchCollection

side = 100
cells = [(i, j) for i in range(side) for j in range(side)]
colors = plt.get_cmap("viridis")(np.array([((i * 7 + j * 3) % 11) / 10 for i, j in cells]))


def render(use_collection):
    fig, ax = plt.subplots(figsize=(4, 4), dpi=60)
    rects = [Rectangle((i, j), 0.9, 0.9) for i, j in cells]
    if use_collection:
        pc = PatchCollection(rects, facecolors=colors, edgecolors="none")
        ax.add_collection(pc)
    else:
        for r, c in zip(rects, colors):
            r.set_facecolor(c)
            r.set_edgecolor("none")
            ax.add_patch(r)
    ax.set_xlim(0, side)
    ax.set_ylim(0, side)
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    n = len(ax.patches), len(ax.collections)
    plt.close(fig)
    buf.seek(0)
    return plt.imread(buf), n


a, na = render(False)
b, nb = render(True)
print(na, nb, a.shape == b.shape)
diff = np.abs(a - b)
print(round(float(diff.mean()), 5), int((diff.sum(axis=2) > 0.05).sum()))
