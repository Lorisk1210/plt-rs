# 5,000 lines drawn as separate Line2D objects and as one LineCollection; artist counts, limits and image difference.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

k = 5000
x = np.linspace(0, 1, 20)
ys = [np.sin(2 * np.pi * (x + j / k)) * (j / k) for j in range(k)]


def render(collection):
    fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
    if collection:
        lc = LineCollection([np.c_[x, y] for y in ys], colors="k", linewidths=0.2, alpha=0.05)
        ax.add_collection(lc)
        ax.autoscale_view()
    else:
        for y in ys:
            ax.plot(x, y, color="k", lw=0.2, alpha=0.05)
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    counts = (len(ax.lines), len(ax.collections), [round(v, 3) for v in ax.get_ylim()])
    plt.close(fig)
    buf.seek(0)
    return plt.imread(buf), counts


a, ca = render(False)
b, cb = render(True)
print(ca, cb)
print(round(float(np.abs(a - b).mean()), 4), round(float(a[..., 0].mean()), 4), round(float(b[..., 0].mean()), 4))
