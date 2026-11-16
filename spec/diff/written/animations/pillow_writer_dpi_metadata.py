# Save an animation with explicit dpi, savefig_kwargs and a progress callback.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation, PillowWriter

fig, ax = plt.subplots(figsize=(3, 3))
img = ax.imshow(np.zeros((10, 10)), vmin=0, vmax=1, cmap="Greens")
progress = []


def update(i):
    data = np.zeros((10, 10))
    data[: i + 1, : i + 1] = 1
    img.set_data(data)
    return (img,)


ani = FuncAnimation(fig, update, frames=5)
ani.save("fill.gif", writer=PillowWriter(fps=2), dpi=40, savefig_kwargs={"facecolor": "lightgray"},
         progress_callback=lambda i, n: progress.append((i, n)))
print(progress)
from PIL import Image
with Image.open("fill.gif") as im:
    print(im.size, im.n_frames)
