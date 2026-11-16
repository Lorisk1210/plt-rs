# Figure settings read from a config dict (size, dpi, layout, facecolor) passed through to figure/subplots/savefig.
import numpy as np
import matplotlib.pyplot as plt

CONFIG = {"figure": {"figsize": (8, 4), "dpi": 90, "layout": "constrained", "facecolor": "#fdfdfd"},
          "subplots": {"nrows": 1, "ncols": 2, "sharey": True},
          "savefig": {"dpi": 60, "bbox_inches": "tight", "facecolor": "auto"}}
fig = plt.figure(**CONFIG["figure"])
axs = fig.subplots(**CONFIG["subplots"])
axs[0].plot(np.arange(10), np.arange(10) ** 0.5)
axs[1].bar(range(5), [3, 1, 4, 1, 5])
fig.suptitle("from config")
fig.savefig("configured.png", **CONFIG["savefig"])
print(fig.get_size_inches().tolist(), fig.dpi, type(fig.get_layout_engine()).__name__, fig.get_facecolor())
print(axs[0].get_shared_y_axes().joined(axs[0], axs[1]))
