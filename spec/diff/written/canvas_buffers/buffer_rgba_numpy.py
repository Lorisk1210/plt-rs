# Get a figure as a numpy RGB array (for OpenCV/ML pipelines) via canvas.buffer_rgba.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
ax.plot([0, 1, 2], [0, 1, 0], color="red", lw=4)
ax.set_facecolor("black")
fig.canvas.draw()
rgba = np.asarray(fig.canvas.buffer_rgba())
rgb = rgba[..., :3]
print(rgba.shape, rgba.dtype)
red = (rgb[..., 0] > 200) & (rgb[..., 1] < 80) & (rgb[..., 2] < 80)
print("red pixels", int(red.sum()) > 50)
print(tuple(int(v) for v in rgba[0, 0]))
plt.imsave("from_buffer.png", rgb)
