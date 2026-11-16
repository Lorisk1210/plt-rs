# Four views of the same 3-D surface in subplots (elev/azim per panel) with proj_type ortho.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 30)
X, Y = np.meshgrid(x, x)
Z = X * np.exp(-X ** 2 - Y ** 2)
fig, axs = plt.subplots(2, 2, figsize=(8, 7), subplot_kw=dict(projection="3d"))
views = [(30, -60), (0, 0), (90, -90), (20, 120)]
for ax, (e, a) in zip(axs.flat, views):
    ax.plot_surface(X, Y, Z, cmap="viridis")
    ax.view_init(e, a)
    ax.set_title(f"elev={e}, azim={a}", fontsize=9)
axs[1, 1].set_proj_type("ortho")
print([ax.elev for ax in axs.flat])
fig.savefig("views.png")
