# Inspect legend handles' types for each kind of artist (line, scatter, bar, errorbar, fill, stem, hist step).
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot([0, 1], [0, 1], label="line")
ax.scatter([0.5], [0.2], label="scatter")
ax.bar([0.2], [0.3], width=0.1, label="bar")
ax.errorbar([0.7], [0.5], yerr=0.1, fmt="s", label="errorbar")
ax.fill_between([0, 1], [0.8, 0.9], [1, 1.1], alpha=0.3, label="fill")
ax.stem([0.9], [0.4], label="stem")
ax.hist([0.1, 0.2, 0.2, 0.3], bins=3, histtype="step", label="hist step")
leg = ax.legend(fontsize=8)
for t, h in zip(leg.get_texts(), leg.legend_handles):
    print(t.get_text(), type(h).__name__)
print(len(ax.get_legend_handles_labels()[0]))
fig.savefig("handles.png", dpi=50)
