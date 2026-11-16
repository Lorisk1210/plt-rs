# Render a chart to an RGBA array, measure its non-white area, and embed that array as an image in a summary figure.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
ax.bar(["a", "b", "c"], [3, 1, 2], color="C3")
ax.axis("off")
fig.canvas.draw()
arr = np.asarray(fig.canvas.buffer_rgba()).copy()
ink = (arr[..., :3] < 250).any(axis=2).mean()
plt.close(fig)
summary, (left, right) = plt.subplots(1, 2, figsize=(8, 3))
left.imshow(arr)
left.set_title("embedded render")
left.axis("off")
right.bar(["ink", "paper"], [ink, 1 - ink], color=["k", "0.9"])
right.set_ylim(0, 1)
print(arr.shape, arr.dtype, round(float(ink), 4))
summary.savefig("summary.png", dpi=50)
