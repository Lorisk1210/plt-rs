# Old code uses tostring_argb; convert ARGB to RGBA by rolling the channel axis.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
ax.fill_between([0, 1], [0, 1], color="green")
fig.canvas.draw()
w, h = fig.canvas.get_width_height()
argb = np.frombuffer(fig.canvas.tostring_argb(), dtype=np.uint8).reshape(h, w, 4)
rgba = np.roll(argb, -1, axis=2)
print(argb.shape, argb[0, 0].tolist(), rgba[0, 0].tolist())
same = np.array_equal(rgba, np.asarray(fig.canvas.buffer_rgba()))
print("equal to buffer_rgba:", same)
plt.imsave("argb_converted.png", rgba)
