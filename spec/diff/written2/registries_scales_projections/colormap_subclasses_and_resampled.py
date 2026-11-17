# Colormap objects in depth: LinearSegmentedColormap from segment data, resampled, reversed, with_extremes, ListedColormap ops.
import numpy as np
from matplotlib.colors import LinearSegmentedColormap, ListedColormap, Colormap
import matplotlib as mpl

cdict = {"red": [(0.0, 0.0, 0.0), (0.5, 1.0, 1.0), (1.0, 1.0, 1.0)],
         "green": [(0.0, 0.0, 0.0), (0.25, 0.0, 0.0), (0.75, 1.0, 1.0), (1.0, 1.0, 1.0)],
         "blue": [(0.0, 0.0, 0.0), (0.5, 0.0, 0.0), (1.0, 1.0, 1.0)]}
heat = LinearSegmentedColormap("usr2_heat", cdict, N=64, gamma=1.2)
vals = np.array([0.0, 0.25, 0.5, 0.75, 1.0, np.nan, -0.1, 1.1])
print(np.round(heat(vals), 6).tolist())
print(heat.N, heat.name, round(heat._gamma, 3), heat.is_gray())
small = heat.resampled(5)
print(small.N, np.round(small(np.linspace(0, 1, 5)), 6).tolist())
rev = heat.reversed()
print(rev.name, np.round(rev(0.25), 6).tolist())
ext = heat.with_extremes(bad="0.5", under="cyan", over="magenta")
print(np.round(ext(np.array([np.nan, -1.0, 2.0])), 6).tolist())
lst = ListedColormap(["red", "green", "blue"], name="rgb")
print(lst.N, lst.colors, np.round(lst(np.array([0.0, 0.4, 0.99])), 3).tolist())
print(np.round(lst([0, 1, 2]), 3).tolist(), lst(1.0, bytes=True), lst(0.5, alpha=0.25))
print(isinstance(heat, Colormap), heat == heat.copy(), heat.copy() is heat)
viridis = mpl.colormaps["viridis"]
print(np.round(viridis.resampled(3)([0.0, 0.5, 1.0]), 6).tolist(), viridis.reversed().name)
gray = LinearSegmentedColormap.from_list("g", ["black", "white"], N=3)
print(np.round(gray(np.arange(3)), 6).tolist(), gray.is_gray())
print(np.round(heat(np.array([0.1, 0.9]), alpha=np.array([0.2, 0.8])), 6).tolist())
print(np.round(heat._lut[:2], 6).tolist() if heat._isinit else "lazy", heat._i_bad, heat._i_over, heat._i_under)
heat2 = heat.copy()
heat2.set_bad("red")
print(np.round(heat2.get_bad(), 3).tolist(), np.round(heat.get_bad(), 3).tolist())
