# For five scenarios save a heat map with shared color limits and a colorbar, plus one summary figure of all five.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:40j, 0:1:60j]
scenarios = {f"S{k}": np.sin((k + 2) * x) * np.cos(3 * y) * (1 + k / 4) for k in range(5)}
vmin = min(v.min() for v in scenarios.values())
vmax = max(v.max() for v in scenarios.values())
for name, field in scenarios.items():
    fig, ax = plt.subplots(figsize=(5, 3.5))
    im = ax.imshow(field, vmin=vmin, vmax=vmax, cmap="RdBu_r", origin="lower")
    fig.colorbar(im, ax=ax, extend="both")
    ax.set_title(name)
    fig.savefig(f"map_{name}.png", dpi=40)
    plt.close(fig)
fig, axs = plt.subplots(1, 5, figsize=(14, 3), layout="constrained")
for ax, (name, field) in zip(axs, scenarios.items()):
    im = ax.imshow(field, vmin=vmin, vmax=vmax, cmap="RdBu_r", origin="lower")
    ax.set_title(name)
    ax.axis("off")
cb = fig.colorbar(im, ax=axs, shrink=0.8)
print(round(vmin, 4), round(vmax, 4), cb.vmin == im.norm.vmin)
fig.savefig("all_maps.png", dpi=40)
