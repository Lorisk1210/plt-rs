# Annotations with mixed coordinate systems: xycoords='data', 'axes fraction', 'figure pixels', offset points.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(np.arange(10), np.arange(10) ** 2)
ax.annotate("data point", xy=(5, 25), xycoords="data", xytext=(-60, 30), textcoords="offset points",
            arrowprops=dict(arrowstyle="-|>"))
ax.annotate("axes corner", xy=(1, 0), xycoords="axes fraction", ha="right", va="bottom")
ax.annotate("figure pixels", xy=(10, 10), xycoords="figure pixels", fontsize=8)
ax.annotate("fraction", xy=(0.5, 0.95), xycoords="figure fraction", ha="center")
ax.annotate("mixed", xy=(2, 0.5), xycoords=("data", "axes fraction"), color="green")
ax.annotate("", xy=(8, 64), xytext=(8, 20), arrowprops=dict(arrowstyle="<->", color="purple"))
for t in ax.texts:
    print(t.get_text(), t.xycoords if hasattr(t, "xycoords") else None)
fig.savefig("coords.png")
