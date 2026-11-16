# Parametric curves (Lissajous, rose, spiral) in a row with equal aspect.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 2 * np.pi, 1000)
fig, axs = plt.subplots(1, 3, figsize=(10, 3.5))
axs[0].plot(np.sin(3 * t), np.sin(4 * t + np.pi / 2))
axs[0].set_title("Lissajous 3:4")
r = np.cos(5 * t)
axs[1].plot(r * np.cos(t), r * np.sin(t), color="C3")
axs[1].set_title("rose k=5")
s = np.linspace(0, 6 * np.pi, 1000)
axs[2].plot(s * np.cos(s), s * np.sin(s), color="C2")
axs[2].set_title("Archimedean spiral")
for ax in axs:
    ax.set_aspect("equal")
    ax.axis("off")
print(len(t))
fig.savefig("parametric.png")
