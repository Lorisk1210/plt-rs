# Scatter coloured by a value spanning decades, with a LogNorm colour bar and custom tick format.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

np.random.seed(4)
x = np.random.rand(200)
y = np.random.rand(200)
mass = 10 ** np.random.uniform(0, 4, 200)
fig, ax = plt.subplots(figsize=(5.5, 4))
sc = ax.scatter(x, y, c=mass, norm=LogNorm(vmin=1, vmax=1e4), cmap="plasma", s=18)
cb = plt.colorbar(sc, ax=ax, format="%.0e")
cb.set_label("mass [kg]")
ax.set_xlabel("x")
ax.set_ylabel("y")
print(type(cb.locator).__name__, cb.norm.vmin, cb.norm.vmax)
fig.savefig("masses.png")
