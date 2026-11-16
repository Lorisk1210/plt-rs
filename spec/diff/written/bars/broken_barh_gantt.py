# Simple Gantt chart of project tasks with broken_barh and day labels.
import matplotlib.pyplot as plt

tasks = [("design", [(0, 5)]), ("build", [(4, 10), (16, 3)]), ("test", [(12, 6)]), ("launch", [(20, 1)])]
fig, ax = plt.subplots(figsize=(7, 3))
for i, (name, spans) in enumerate(tasks):
    ax.broken_barh(spans, (i - 0.35, 0.7), facecolors=f"C{i}")
    for start, length in spans:
        ax.text(start + length / 2, i, f"{length}d", ha="center", va="center", color="white", fontsize=8)
ax.set_yticks(range(len(tasks)), [t[0] for t in tasks])
ax.invert_yaxis()
ax.set_xlabel("day")
ax.grid(axis="x", alpha=0.4)
fig.tight_layout()
print(len(ax.collections))
fig.savefig("gantt.png")
