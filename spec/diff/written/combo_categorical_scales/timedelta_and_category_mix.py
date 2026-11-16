# Task durations: categories on y, start offsets in hours via broken_barh, and a duration histogram in minutes.
import numpy as np
import matplotlib.pyplot as plt

tasks = {"fetch": [(0, 1.5), (5, 1)], "clean": [(1.5, 2)], "train": [(3.5, 6)], "eval": [(9.5, 1), (11, 0.5)]}
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 3.5), gridspec_kw={"width_ratios": [2, 1]})
for i, (name, spans) in enumerate(tasks.items()):
    a.broken_barh(spans, (i - 0.3, 0.6), color=f"C{i}")
a.set_yticks(range(len(tasks)), list(tasks))
a.set_xlabel("hours since start")
a.invert_yaxis()
durations = np.array([d for spans in tasks.values() for _, d in spans]) * 60
b.hist(durations, bins=[0, 30, 60, 120, 240, 480], edgecolor="k")
b.set_xscale("log")
b.set_xlabel("minutes")
print(durations.tolist(), [t.get_text() for t in a.get_yticklabels()])
fig.savefig("tasks.png", dpi=50)
