# Read the colour bar's ticks after drawing and draw matching contour lines and horizontal guides on a histogram.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:80j, 0:1:80j]
z = np.sin(5 * x) * np.cos(4 * y) * 10
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4.5))
im = a.imshow(z, cmap="coolwarm", origin="lower")
cb = fig.colorbar(im, ax=a)
fig.canvas.draw()
ticks = [t for t in cb.get_ticks() if z.min() <= t <= z.max()]
a.contour(z, levels=ticks, colors="k", linewidths=0.5)
b.hist(z.ravel(), bins=40, orientation="horizontal")
for t in ticks:
    b.axhline(t, color="k", lw=0.5, ls=":")
print([round(float(t), 3) for t in ticks])
fig.savefig("cbar_ticks.png", dpi=50)
