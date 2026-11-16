# Take a line's data (get_xydata), transform it to display and back, and draw shifted copies in points with offsets.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms

fig, ax = plt.subplots(figsize=(6, 4), dpi=100)
line, = ax.plot(np.linspace(0, 1, 10), np.linspace(0, 1, 10) ** 2, "o-")
fig.canvas.draw()
xy = line.get_xydata()
disp = ax.transData.transform(xy)
back = ax.transData.inverted().transform(disp + [5, -5])
ax.plot(back[:, 0], back[:, 1], "s", color="C1", ms=4)
for dy in (10, 20):
    tr = ax.transData + mtransforms.ScaledTranslation(0, dy / 72, fig.dpi_scale_trans)
    ax.plot(*xy.T, color="C2", alpha=0.5, transform=tr)
print(xy.shape, np.round(disp[1], 2).tolist(), np.round(back[1], 4).tolist())
fig.savefig("shifted.png", dpi=50)
