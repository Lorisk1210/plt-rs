# Combine legend entries from an axes, its twinx and an inset into one legend, deduplicating labels.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(12)
fig, ax = plt.subplots(figsize=(8, 4.5))
ax.bar(x, x + 3, color="0.7", label="volume")
tw = ax.twinx()
tw.plot(x, np.sqrt(x), "C3o-", label="price")
tw.plot(x, np.sqrt(x) * 0.9, "C3:", label="price")
ins = ax.inset_axes([0.05, 0.6, 0.3, 0.3])
ins.plot(x, x ** 2, "C0", label="model")
handles, labels = [], []
for a in (ax, tw, ins):
    for h, l in zip(*a.get_legend_handles_labels()):
        if l not in labels:
            handles.append(h); labels.append(l)
ax.legend(handles, labels, loc="upper right")
print(labels, [type(h).__name__ for h in handles])
fig.savefig("combined_legend.png", dpi=50)
