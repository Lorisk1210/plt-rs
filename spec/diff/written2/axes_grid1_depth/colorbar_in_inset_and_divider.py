# Colorbars placed three ways next to the same image: an append_axes cax, an inset_axes cax, and axes_grid1's own colorbar.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable
from mpl_toolkits.axes_grid1.inset_locator import inset_axes

data = np.outer(np.linspace(-1, 1, 30), np.linspace(0, 2, 40))
fig, axs = plt.subplots(1, 3, figsize=(9, 3), dpi=60)
ims = [ax.imshow(data, cmap="RdBu", vmin=-2, vmax=2) for ax in axs]
cax0 = make_axes_locatable(axs[0]).append_axes("right", size="6%", pad=0.05)
cb0 = fig.colorbar(ims[0], cax=cax0)
cax1 = inset_axes(axs[1], width="5%", height="80%", loc="center left", bbox_to_anchor=(1.02, 0, 1, 1),
                  bbox_transform=axs[1].transAxes, borderpad=0)
cb1 = fig.colorbar(ims[1], cax=cax1, extend="both")
cax2 = inset_axes(axs[2], width="90%", height="6%", loc="lower center", borderpad=-2)
cb2 = fig.colorbar(ims[2], cax=cax2, orientation="horizontal", ticks=[-2, 0, 2])
fig.canvas.draw()
for cb in (cb0, cb1, cb2):
    print(cb.orientation, [round(v, 3) for v in cb.ax.get_position().bounds], [round(t, 2) for t in cb.get_ticks()])
print(cb1.extend, len(cb1._extend_patches), cb2.ax.get_xticklabels()[0].get_text())
ims[0].set_clim(-1, 1)
fig.canvas.draw()
print([round(t, 2) for t in cb0.get_ticks()], [round(v, 2) for v in cb0.ax.get_ylim()])
fig.savefig("colorbars_grid1.png")
cb2.set_ticks([-1.5, 0, 1.5], labels=["low", "zero", "high"])
fig.canvas.draw()
print([t.get_text() for t in cb2.ax.get_xticklabels()])
cb1.remove()
print(len(fig.axes), cax1 in fig.axes)
