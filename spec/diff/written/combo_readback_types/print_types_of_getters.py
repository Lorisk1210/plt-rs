# A script printing the types and values of common getters (users inspect what they get) for line, bar, text, image.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
line, = ax.plot([1, 2, 3], [4, 5, 6], "o-", lw=2.5, alpha=0.5)
bar = ax.bar([1], [2])[0]
txt = ax.text(1, 1, "hi", fontsize=12, rotation=45)
im = ax.imshow(np.eye(3), extent=(0, 1, 0, 1))
probes = {
    "line.get_xdata": line.get_xdata(), "line.get_linewidth": line.get_linewidth(), "line.get_alpha": line.get_alpha(),
    "line.get_color": line.get_color(), "line.get_marker": line.get_marker(), "line.get_markersize": line.get_markersize(),
    "bar.get_height": bar.get_height(), "bar.get_facecolor": bar.get_facecolor(), "bar.get_xy": bar.get_xy(),
    "txt.get_fontsize": txt.get_fontsize(), "txt.get_rotation": txt.get_rotation(), "txt.get_position": txt.get_position(),
    "im.get_extent": im.get_extent(), "im.get_clim": im.get_clim(), "ax.get_xlim": ax.get_xlim(), "fig.get_dpi": fig.get_dpi(),
}
for name, value in probes.items():
    print(name, type(value).__name__, value)
fig.savefig("types.png", dpi=40)
