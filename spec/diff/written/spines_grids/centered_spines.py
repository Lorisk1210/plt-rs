# Math-textbook axes crossing at the origin, with arrows at the ends of the spines.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 200)
fig, ax = plt.subplots(figsize=(5, 4))
ax.plot(x, x ** 3 - 3 * x, label="f(x)")
ax.spines["left"].set_position("zero")
ax.spines["bottom"].set_position("zero")
ax.spines[["top", "right"]].set_visible(False)
ax.plot(1, 0, ">k", transform=ax.get_yaxis_transform(), clip_on=False)
ax.plot(0, 1, "^k", transform=ax.get_xaxis_transform(), clip_on=False)
ax.set_ylim(-6, 6)
ax.legend(loc="upper left")
print(ax.spines["left"].get_position(), ax.spines["bottom"].get_position())
fig.savefig("origin.png")
