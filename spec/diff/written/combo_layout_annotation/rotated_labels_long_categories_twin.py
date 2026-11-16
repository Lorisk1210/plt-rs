# Long rotated category labels with a twin axis, value labels and a reference line; tight save.
import numpy as np
import matplotlib.pyplot as plt

cats = [f"department of {n}" for n in ["finance", "human resources", "research", "operations", "marketing", "legal"]]
budget = np.array([120, 45, 200, 160, 90, 30])
share = budget / budget.sum() * 100
fig, ax = plt.subplots(figsize=(8, 4.5))
bars = ax.bar(cats, budget, color="C0")
ax.bar_label(bars, fmt="%d k")
ax.set_xticks(range(len(cats)), cats, rotation=35, ha="right", rotation_mode="anchor")
tw = ax.twinx()
tw.plot(range(len(cats)), np.cumsum(share), "ko-")
tw.axhline(80, color="r", ls="--")
tw.set_ylim(0, 105)
fig.savefig("departments.png", dpi=50, bbox_inches="tight")
print([round(v, 2) for v in np.cumsum(share)], ax.get_xticklabels()[0].get_rotation())
