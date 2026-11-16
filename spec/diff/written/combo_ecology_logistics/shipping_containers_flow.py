# Container flows between ports: a chord-like arc diagram on a circle with line widths by volume and a volume table.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.patches import PathPatch

ports = ["SHA", "SIN", "RTM", "LAX", "DXB", "HAM"]
rng = np.random.default_rng(215)
flows = rng.integers(0, 50, (6, 6)); np.fill_diagonal(flows, 0)
ang = np.linspace(0, 2 * np.pi, 6, endpoint=False)
pos = np.c_[np.cos(ang), np.sin(ang)]
fig, (a, t) = plt.subplots(1, 2, figsize=(12, 5.5), gridspec_kw={"width_ratios": [1.2, 1]})
for i in range(6):
    for j in range(i + 1, 6):
        v = flows[i, j] + flows[j, i]
        path = Path([pos[i], (0, 0), pos[j]], [Path.MOVETO, Path.CURVE3, Path.CURVE3])
        a.add_patch(PathPatch(path, fill=False, lw=v / 15, alpha=0.6, color=plt.cm.tab10(i)))
for (x, y), p in zip(pos, ports):
    a.plot(x, y, "ko"); a.text(1.12 * x, 1.12 * y, p, ha="center", va="center")
a.set_aspect("equal"); a.axis("off"); a.set_xlim(-1.3, 1.3); a.set_ylim(-1.3, 1.3)
t.axis("off")
t.table(cellText=flows, rowLabels=ports, colLabels=ports, loc="center").set_fontsize(8)
print(int(flows.sum()), len(a.patches))
fig.savefig("flows.png", dpi=50)
