# Save only the left panel of a two-panel figure, and a version with a legend outside that would be clipped.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox

t = np.linspace(0, 2 * np.pi, 100)
fig, (left, right) = plt.subplots(1, 2, figsize=(8, 3))
left.plot(t, np.sin(t), label="sin")
left.plot(t, np.cos(t), label="cos")
right.scatter(np.cos(t), np.sin(t), c=t, s=8)
right.set_aspect("equal")
left.legend(loc="upper left", bbox_to_anchor=(1.0, 1.0))

extent = left.get_window_extent().transformed(fig.dpi_scale_trans.inverted())
print(np.round(extent.bounds, 3))
fig.savefig("left_only.png", bbox_inches=extent.expanded(1.25, 1.3))
fig.savefig("box_inches.png", bbox_inches=Bbox([[0.5, 0.2], [4.5, 2.8]]))
fig.savefig("tight.png", bbox_inches="tight")
fig.savefig("tight_extra.png", bbox_inches="tight",
            bbox_extra_artists=[left.get_legend()], pad_inches=0.3)
