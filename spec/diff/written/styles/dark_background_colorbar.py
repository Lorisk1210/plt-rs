# Dark background style with an image, colour bar and legend; savefig keeps the dark face.
import numpy as np
import matplotlib.pyplot as plt

plt.style.use("dark_background")
y, x = np.mgrid[-2:2:100j, -2:2:100j]
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
im = a.imshow(np.exp(-(x ** 2 + y ** 2)) * np.cos(4 * x), cmap="magma")
fig.colorbar(im, ax=a)
b.plot(np.arange(20), np.arange(20) ** 0.5, label="sqrt", color="cyan")
b.legend()
fig.savefig("dark.png")
print(plt.rcParams["figure.facecolor"], plt.rcParams["text.color"])
