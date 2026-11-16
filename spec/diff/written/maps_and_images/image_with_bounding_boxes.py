# Object-detection style overlay: rectangles with labels and scores on an image.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

img = np.zeros((120, 160, 3))
img[..., 1] = np.linspace(0.2, 0.6, 160)
img[30:70, 20:60] = [0.9, 0.3, 0.3]
img[50:100, 90:140] = [0.3, 0.3, 0.9]
boxes = [(20, 30, 40, 40, "cat", 0.92), (90, 50, 50, 50, "dog", 0.81), (5, 5, 30, 20, "bird", 0.42)]
fig, ax = plt.subplots(figsize=(6, 4.5))
ax.imshow(img)
for x, y, w, h, label, score in boxes:
    color = "yellow" if score > 0.5 else "gray"
    ax.add_patch(Rectangle((x, y), w, h, fill=False, ec=color, lw=2))
    ax.text(x, y - 2, f"{label} {score:.2f}", color="k", fontsize=8, bbox=dict(fc=color, ec="none", pad=1))
ax.axis("off")
print(len(ax.patches))
fig.savefig("detections.png")
