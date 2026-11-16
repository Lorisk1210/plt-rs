# School timetable: a weekday x period grid of coloured subject rectangles with text, and a subject hours bar chart.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

subjects = ["math", "english", "science", "art", "sport", "history"]
rng = np.random.default_rng(237)
table = rng.choice(len(subjects), (5, 6))
colors = dict(zip(subjects, plt.cm.Set3.colors))
fig, (a, b) = plt.subplots(1, 2, figsize=(13, 4.5), gridspec_kw={"width_ratios": [2, 1]})
for d in range(5):
    for p in range(6):
        s = subjects[table[d, p]]
        a.add_patch(Rectangle((d, -p), 0.95, 0.9, color=colors[s]))
        a.text(d + 0.47, -p + 0.45, s, ha="center", va="center", fontsize=8)
a.set_xlim(0, 5); a.set_ylim(-5.1, 1)
a.set_xticks(np.arange(5) + 0.47, ["Mon", "Tue", "Wed", "Thu", "Fri"])
a.set_yticks(-np.arange(6) + 0.45, [f"P{p + 1}" for p in range(6)])
counts = np.bincount(table.ravel(), minlength=6)
b.barh(subjects, counts, color=[colors[s] for s in subjects], edgecolor="k")
print(counts.tolist(), len(a.texts))
fig.savefig("timetable.png", dpi=50)
