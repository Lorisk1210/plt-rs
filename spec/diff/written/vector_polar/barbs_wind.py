# Meteorological wind barbs on a small grid, flipped in the southern half.
import numpy as np
import matplotlib.pyplot as plt

x, y = np.meshgrid(np.arange(-5, 6, 2), np.arange(-5, 6, 2))
u = 10 + 5 * np.cos(x / 3)
v = 15 * np.sin(y / 4)
fig, ax = plt.subplots(figsize=(5, 5))
ax.barbs(x, y, u, v, length=6, flip_barb=y < 0, barbcolor="navy", flagcolor="red",
         sizes=dict(emptybarb=0.2))
ax.set_xlim(-6, 6)
ax.set_ylim(-6, 6)
ax.set_title("wind barbs (knots)")
print(len(ax.collections))
fig.savefig("barbs.png")
