# Save an array as an image file with imsave, read it back with imread and compare shapes.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:64j, 0:1:96j]
arr = np.sin(10 * x) * np.cos(10 * y)
plt.imsave("pattern.png", arr, cmap="plasma", vmin=-1, vmax=1)
plt.imsave("pattern_gray.png", arr, cmap="gray", origin="lower")
back = plt.imread("pattern.png")
print(back.shape, back.dtype, round(float(back[..., :3].mean()), 4))
fig, ax = plt.subplots(figsize=(4, 3))
ax.imshow(back)
ax.set_title("read back")
fig.savefig("readback.png")
gray = plt.imread("pattern_gray.png")
print(gray.shape)
