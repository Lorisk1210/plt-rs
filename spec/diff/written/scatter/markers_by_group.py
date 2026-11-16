# Different markers and colours per group in a loop, with a legend, hollow markers for one group.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
groups = {"control": ("o", "C0"), "dose 1": ("s", "C1"), "dose 2": ("^", "C2"), "dose 3": ("D", "C3")}
fig, ax = plt.subplots(figsize=(6, 4))
for k, (name, (marker, color)) in enumerate(groups.items()):
    x = rng.normal(k, 0.5, 20)
    y = rng.normal(k * 0.5, 0.4, 20)
    face = "none" if name == "control" else color
    ax.scatter(x, y, marker=marker, facecolors=face, edgecolors=color, label=name, s=36)
ax.legend(title="group", loc="upper left")
ax.set_xlabel("x")
ax.set_ylabel("y")
print([h.get_label() for h in ax.collections])
fig.savefig("groups.png")
