# Show a grayscale image coloured by a colormap where alpha comes from another array (RGBA build).
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[-1:1:200j, -1:1:200j]
value = np.cos(5 * x) * np.sin(5 * y)
confidence = np.clip(1 - np.hypot(x, y), 0, 1)
rgba = plt.cm.RdBu_r(plt.Normalize(-1, 1)(value))
rgba[..., 3] = confidence
fig, ax = plt.subplots(figsize=(5, 5))
ax.set_facecolor("0.85")
ax.imshow(rgba, extent=(-1, 1, -1, 1))
ax.set_title("alpha = confidence")
print(rgba.shape, round(float(rgba[..., 3].mean()), 4))
fig.savefig("confidence.png")
