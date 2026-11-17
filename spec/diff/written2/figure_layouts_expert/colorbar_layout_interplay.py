# Colorbars stealing space: ax=list of axes, use_gridspec, fraction/pad/aspect/anchor/panchor; positions of parents and cbars.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for kw in [dict(), dict(fraction=0.3, pad=0.1), dict(aspect=5, shrink=0.5, anchor=(0, 1), panchor=(0, 0)),
           dict(location="left"), dict(location="top", orientation="horizontal")]:
    fig, axs = plt.subplots(1, 2, figsize=(6, 3), dpi=60)
    im = axs[0].imshow(np.eye(3))
    cb = fig.colorbar(im, ax=list(axs), **kw)
    fig.canvas.draw()
    print(sorted(kw), [[round(v, 3) for v in a.get_position().bounds] for a in axs], [round(v, 3) for v in cb.ax.get_position().bounds])
    plt.close(fig)
fig, ax = plt.subplots(figsize=(4, 3))
cb = fig.colorbar(ax.imshow(np.eye(3)), ax=ax, use_gridspec=True)
fig.canvas.draw()
print([round(v, 3) for v in ax.get_position().bounds], cb.ax.get_subplotspec() is not None)
cb.remove()
fig.canvas.draw()
print([round(v, 3) for v in ax.get_position().bounds], len(fig.axes))
fig2, axs2 = plt.subplots(2, 2, layout="constrained")
cb2 = fig2.colorbar(axs2[0, 0].imshow(np.eye(2)), ax=axs2[:, 1], location="right")
fig2.canvas.draw()
print([round(v, 3) for v in cb2.ax.get_position().bounds], [round(v, 3) for v in axs2[0, 1].get_position().bounds])
cb2.ax.set_ylabel("value")
fig2.canvas.draw()
print([round(v, 3) for v in cb2.ax.get_position().bounds])
print(cb2.orientation, cb2.ax.get_label())
fig2.savefig("cbar_layout.png")
