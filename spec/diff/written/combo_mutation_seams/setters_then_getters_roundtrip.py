# Change many properties after creation via set_*, then read each back and print types and values (line, patch, text).
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
line, = ax.plot([0, 1, 2], [0, 1, 0])
patch = ax.bar([1], [0.5])[0]
text = ax.text(0.5, 0.5, "x")
line.set(color=(0.1, 0.2, 0.3), linewidth=np.float32(2.5), marker="D", markersize=np.int64(9), alpha=np.float64(0.7),
         dashes=(4, 1), zorder=np.int32(5), markevery=2)
patch.set(facecolor="#ff000080", edgecolor=np.array([0, 0, 1.0]), linewidth=3, hatch="//", width=0.3)
text.set(fontsize=np.float64(14.0), rotation=np.int64(450), color="C2", position=(np.float64(0.2), 0.3))
for name in ["color", "linewidth", "marker", "markersize", "alpha", "linestyle", "zorder", "markevery"]:
    v = getattr(line, f"get_{name}")()
    print("line", name, type(v).__name__, v)
for name in ["facecolor", "edgecolor", "linewidth", "hatch", "width"]:
    v = getattr(patch, f"get_{name}")()
    print("patch", name, type(v).__name__, v)
for name in ["fontsize", "rotation", "color", "position"]:
    v = getattr(text, f"get_{name}")()
    print("text", name, type(v).__name__, v)
fig.savefig("roundtrip.png", dpi=50)
