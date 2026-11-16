# Legend with mixed handles (sizes via legend_elements, line styles, filled areas) in columns, placed below with a title.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(220)
fig, ax = plt.subplots(figsize=(8, 5))
sc = ax.scatter(rng.random(30), rng.random(30), s=rng.uniform(10, 300, 30), alpha=0.5)
l1, = ax.plot([0, 1], [0.2, 0.8], "k--", label="trend")
f = ax.fill_between([0, 1], [0.1, 0.5], [0.3, 0.9], alpha=0.2, label="band")
hs, ls = sc.legend_elements(prop="sizes", num=4, fmt="{x:.0f}")
leg = ax.legend(hs + [l1, f], ls + ["trend", "band"], ncols=3, loc="upper center", bbox_to_anchor=(0.5, -0.1), title="legend")
fig.subplots_adjust(bottom=0.3)
print(ls, len(leg.legend_handles), leg.get_title().get_text())
fig.savefig("mixed_legend.png", dpi=50)
