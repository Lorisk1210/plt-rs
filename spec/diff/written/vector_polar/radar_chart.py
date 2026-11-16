# Radar (spider) chart of two players' skill ratings on a polar axes, closed polygons.
import numpy as np
import matplotlib.pyplot as plt

skills = ["speed", "strength", "stamina", "technique", "vision", "defence"]
a = [8, 6, 7, 9, 8, 4]
b = [6, 9, 8, 5, 6, 8]
angles = np.linspace(0, 2 * np.pi, len(skills), endpoint=False).tolist()
angles += angles[:1]
fig, ax = plt.subplots(figsize=(5, 5), subplot_kw=dict(polar=True))
for name, vals, color in [("Ana", a, "C0"), ("Ben", b, "C1")]:
    vals = vals + vals[:1]
    ax.plot(angles, vals, color=color, lw=2, label=name)
    ax.fill(angles, vals, color=color, alpha=0.2)
ax.set_xticks(angles[:-1], skills)
ax.set_ylim(0, 10)
ax.legend(loc="upper right", bbox_to_anchor=(1.2, 1.1))
fig.savefig("radar.png")
print(len(ax.lines))
