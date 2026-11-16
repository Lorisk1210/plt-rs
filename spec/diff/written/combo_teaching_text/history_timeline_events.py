# History timeline: eras as broken_barh bands, events as stems with alternating label heights, and a period legend.
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

eras = [("Stone", -3000, 1800, "#c7b299"), ("Bronze", -1200, 400, "#cd7f32"), ("Iron", -800, 1100, "#8a8a8a"), ("Medieval", 500, 1000, "#7b68ee")]
events = [(-2560, "pyramid"), (-776, "olympics"), (-221, "unification"), (476, "fall of Rome"), (800, "coronation"), (1066, "conquest")]
fig, ax = plt.subplots(figsize=(11, 4))
for name, start, length, color in eras:
    ax.broken_barh([(start, length)], (-0.3, 0.6), color=color, alpha=0.7)
for k, (year, label) in enumerate(events):
    h = 1 + (k % 3) * 0.6
    ax.vlines(year, 0.3, h, color="k", lw=0.8)
    ax.text(year, h + 0.05, f"{label}\n{abs(year)} {'BCE' if year < 0 else 'CE'}", ha="center", fontsize=8)
ax.legend(handles=[Patch(color=c, alpha=0.7, label=n) for n, _, _, c in eras], loc="lower left", ncols=4)
ax.set_ylim(-1, 2.8); ax.set_yticks([])
ax.set_xlim(-3200, 1600)
print(len(ax.texts), [round(v) for v in ax.get_xlim()])
fig.savefig("timeline.png", dpi=50)
