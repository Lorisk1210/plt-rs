# "Copy" a figure into another as an image: save to a buffer, imread, and imshow it in a grid.
import io
import numpy as np
import matplotlib.pyplot as plt

panels = []
for k in range(3):
    f, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot(np.arange(10) ** (k + 1))
    ax.set_title(f"power {k + 1}")
    buf = io.BytesIO()
    f.savefig(buf, format="png")
    buf.seek(0)
    panels.append(plt.imread(buf))
    plt.close(f)
fig, axs = plt.subplots(1, 3, figsize=(9, 2.5))
for ax, img in zip(axs, panels):
    ax.imshow(img)
    ax.axis("off")
print([p.shape for p in panels])
fig.savefig("collage.png")
