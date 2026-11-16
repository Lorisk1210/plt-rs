# Print colours as returned by various getters: tuples vs arrays, then normalise them with to_hex for a legend.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors

fig, ax = plt.subplots(figsize=(6, 4))
line, = ax.plot([0, 1], color="C1")
patch = ax.bar([0], [1], color="#336699")[0]
sc = ax.scatter([0.5], [0.5], c="green")
fb = ax.fill_between([0, 1], [0, 0.5], color=(0.2, 0.4, 0.6, 0.5))
got = {"line": line.get_color(), "patch face": patch.get_facecolor(), "patch edge": patch.get_edgecolor(),
       "scatter face": sc.get_facecolor(), "fill face": fb.get_facecolor(), "fill edge": fb.get_edgecolor()}
for k, v in got.items():
    print(k, type(v).__name__, np.round(mcolors.to_rgba_array(v), 4).tolist())
hexes = [mcolors.to_hex(mcolors.to_rgba_array(v)[0], keep_alpha=True) if len(mcolors.to_rgba_array(v)) else "none" for v in got.values()]
print(hexes)
ax.legend([line, patch, sc, fb], hexes[:4], fontsize=8)
fig.savefig("rgba.png", dpi=50)
