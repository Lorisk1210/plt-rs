# Horizontal progress bars for goals: a grey track and a coloured fill with percentage text.
import matplotlib.pyplot as plt

goals = {"Steps": (8200, 10000), "Water": (1.6, 2.5), "Sleep": (7.5, 8), "Reading": (12, 30)}
fig, ax = plt.subplots(figsize=(6, 3))
for i, (name, (done, target)) in enumerate(goals.items()):
    frac = min(done / target, 1)
    ax.barh(i, 1, color="0.9", height=0.5)
    ax.barh(i, frac, color="seagreen" if frac >= 0.9 else "orange", height=0.5)
    ax.text(1.02, i, f"{frac:.0%}", va="center")
ax.set_yticks(range(len(goals)), list(goals))
ax.set_xlim(0, 1.15)
ax.invert_yaxis()
ax.axis("on")
ax.spines[:].set_visible(False)
ax.set_xticks([])
print([round(d / t, 3) for d, t in goals.values()])
fig.savefig("progress.png")
