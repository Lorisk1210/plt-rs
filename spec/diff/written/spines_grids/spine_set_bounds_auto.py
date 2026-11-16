# Spines that only span the tick range using set_bounds with the first and last tick after draw.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0.3, 9.7, 40)
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.scatter(x, np.log(x), s=10)
fig.canvas.draw()
xt = ax.get_xticks()
yt = ax.get_yticks()
xt = xt[(xt >= ax.get_xlim()[0]) & (xt <= ax.get_xlim()[1])]
yt = yt[(yt >= ax.get_ylim()[0]) & (yt <= ax.get_ylim()[1])]
ax.spines["bottom"].set_bounds(xt[0], xt[-1])
ax.spines["left"].set_bounds(yt[0], yt[-1])
ax.spines[["top", "right"]].set_visible(False)
print(xt, np.round(yt, 2))
fig.savefig("trimmed.png")
