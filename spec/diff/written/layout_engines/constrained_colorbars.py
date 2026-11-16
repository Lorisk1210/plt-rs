# Constrained layout keeps colour bars and titles from colliding in a 2x2 grid of fields.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[-2:2:80j, -2:2:80j]
fig, axs = plt.subplots(2, 2, figsize=(7, 6), layout="constrained")
fields = [x * np.exp(-x ** 2 - y ** 2), np.sin(3 * x) * np.cos(3 * y), x ** 2 - y ** 2, np.hypot(x, y)]
for ax, field, cmap in zip(axs.flat, fields, ["RdBu_r", "PiYG", "coolwarm", "viridis"]):
    pcm = ax.pcolormesh(x, y, field, cmap=cmap, shading="auto")
    fig.colorbar(pcm, ax=ax, label="value")
    ax.set_title(cmap)
fig.get_layout_engine().set(w_pad=0.1, h_pad=0.1, hspace=0.05, wspace=0.05)
print(type(fig.get_layout_engine()).__name__)
fig.savefig("fields.png")
fig.suptitle("Four fields")
fig.savefig("fields_titled.png", dpi=70)
