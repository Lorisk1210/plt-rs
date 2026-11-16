# Mark crop regions on an image with numbered rectangles and show each crop enlarged in a row underneath.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

y, x = np.mgrid[0:1:200j, 0:1:300j]
img = np.sin(30 * x * y) + np.cos(20 * (x - y))
crops = [(30, 40, 50, 40), (150, 100, 60, 50), (220, 20, 40, 60)]
fig = plt.figure(figsize=(10, 7))
gs = fig.add_gridspec(2, 3, height_ratios=[2, 1])
main = fig.add_subplot(gs[0, :])
main.imshow(img, cmap="cividis")
for k, (cx, cy, w, h) in enumerate(crops):
    main.add_patch(Rectangle((cx, cy), w, h, fill=False, ec=f"C{k}", lw=2))
    main.text(cx, cy - 3, str(k + 1), color=f"C{k}", fontweight="bold")
    sub = fig.add_subplot(gs[1, k])
    sub.imshow(img[cy:cy + h, cx:cx + w], cmap="cividis", vmin=img.min(), vmax=img.max())
    for s in sub.spines.values():
        s.set_edgecolor(f"C{k}")
        s.set_linewidth(3)
    sub.set_xticks([]); sub.set_yticks([])
fig.tight_layout()
print([img[cy:cy + h, cx:cx + w].shape for cx, cy, w, h in crops])
fig.savefig("crops.png", dpi=50)
