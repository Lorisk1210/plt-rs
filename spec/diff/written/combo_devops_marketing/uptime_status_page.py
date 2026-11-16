# Status page: 90-day uptime bars per service (coloured cells), incident annotations, and overall SLA gauge text.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap, BoundaryNorm

rng = np.random.default_rng(208)
services = ["api", "web", "db", "queue"]
uptime = np.clip(1 - rng.exponential(0.002, (4, 90)), 0.9, 1)
uptime[1, 40] = 0.93; uptime[2, 70] = 0.95
cmap = ListedColormap(["#d62728", "#ff7f0e", "#2ca02c"])
norm = BoundaryNorm([0.9, 0.97, 0.995, 1.0001], cmap.N)
fig, ax = plt.subplots(figsize=(12, 3.5))
ax.imshow(uptime, cmap=cmap, norm=norm, aspect="auto")
ax.set_yticks(range(4), services)
ax.set_xlabel("days ago")
ax.set_xticks([0, 30, 60, 89], ["90", "60", "30", "0"])
for (s, d) in [(1, 40), (2, 70)]:
    ax.annotate("incident", (d, s), xytext=(d + 3, s - 0.6), fontsize=7, arrowprops=dict(arrowstyle="->"))
sla = uptime.mean(axis=1)
for k, v in enumerate(sla):
    ax.text(91, k, f"{v:.3%}", va="center", fontsize=8)
ax.set_xlim(-0.5, 97)
print([round(float(v), 5) for v in sla])
fig.savefig("status.png", dpi=50)
