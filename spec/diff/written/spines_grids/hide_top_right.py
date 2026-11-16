# Clean "Tufte-like" axes: hide top and right spines, offset the others, and trim them to the data.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(1, 11)
y = np.array([3, 4, 4.5, 5.2, 5.0, 6.1, 6.8, 7.0, 7.4, 8.1])
fig, ax = plt.subplots(figsize=(5.5, 3.5))
ax.plot(x, y, "o-", color="k", ms=4)
ax.spines[["top", "right"]].set_visible(False)
ax.spines["left"].set_position(("outward", 8))
ax.spines["bottom"].set_position(("outward", 8))
ax.spines["left"].set_bounds(3, 8)
ax.spines["bottom"].set_bounds(1, 10)
ax.set_xticks([1, 4, 7, 10])
ax.set_yticks([3, 5.5, 8])
print(ax.spines["left"].get_position(), ax.spines["top"].get_visible())
fig.savefig("clean.png")
