# f-strings and repr of numpy scalars from getters in titles and labels (np.float64 shows up in repr).
import numpy as np
import matplotlib.pyplot as plt

data = np.array([2.5, 3.75, 1.125, 4.0])
fig, ax = plt.subplots(figsize=(6, 3.5))
bars = ax.bar(range(4), data)
top = max(bars, key=lambda b: b.get_height())
ax.set_title(f"max {top.get_height()} at {top.get_x() + top.get_width() / 2}")
ax.set_xlabel(repr(data.mean()))
ax.set_ylabel(str(data.sum()))
lo, hi = ax.get_ylim()
ax.text(0, hi * 0.9, f"{lo=:.3f} {hi=}")
print(ax.get_title(), "|", ax.get_xlabel(), "|", ax.get_ylabel())
print(ax.texts[0].get_text(), type(top.get_height()).__name__, type(lo).__name__)
fig.savefig("numpy_labels.png", dpi=50)
