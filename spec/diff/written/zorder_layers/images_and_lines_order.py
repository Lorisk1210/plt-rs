# Images default zorder 0 under lines, but an image with zorder 3 covers lines; alpha shows both.
import numpy as np
import matplotlib.pyplot as plt

z = np.outer(np.linspace(0, 1, 20), np.linspace(0, 1, 20))
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.5))
for ax, zo in [(a, 0), (b, 3)]:
    ax.plot([0, 19], [0, 19], color="white", lw=4)
    ax.imshow(z, zorder=zo, alpha=0.8 if zo else 1.0)
    ax.set_title(f"image zorder {zo}")
print(a.images[0].get_zorder(), b.images[0].get_zorder())
fig.savefig("image_order.png")
fig.savefig("images_and_lines_order_small.png", dpi=40)
print(fig.get_size_inches(), len(fig.axes))
