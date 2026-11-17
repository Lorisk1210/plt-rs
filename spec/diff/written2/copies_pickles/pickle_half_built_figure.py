# Pickle a figure at several stages of building (before draw, after draw, with a legend and colorbar) and compare restored state.
import io
import pickle
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def png(fig):
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    return buf.getvalue()


fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
stages = []
ax.plot(np.arange(10), np.arange(10) ** 0.5, label="root")
stages.append(("plotted", pickle.dumps(fig)))
fig.canvas.draw()
stages.append(("drawn", pickle.dumps(fig)))
ax.legend()
im = ax.imshow(np.eye(4), extent=(0, 3, 0, 3))
fig.colorbar(im, ax=ax)
stages.append(("legend+colorbar", pickle.dumps(fig)))
ax.set_xlim(2, 8)
stages.append(("zoomed", pickle.dumps(fig)))
ref = png(fig)
for name, blob in stages:
    f2 = pickle.loads(blob)
    a2 = f2.axes[0]
    print(name, len(f2.axes), [round(v, 2) for v in a2.get_xlim()], a2.get_legend() is not None, len(a2.images),
          type(f2.canvas).__name__, f2.number if hasattr(f2, "number") else None)
last = pickle.loads(stages[-1][1])
print(png(last) == ref, len(blob) > 1000)
print(sorted(plt.get_fignums()))
