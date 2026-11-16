# Read legend entries (texts and handles) and turn them into inline labels next to the lines, then drop the legend.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(1, 10, 40)
fig, ax = plt.subplots(figsize=(7, 4))
for p in (0.5, 1.0, 1.5):
    ax.plot(x, x ** p, label=f"x^{p}")
leg = ax.legend()
pairs = [(t.get_text(), h.get_color()) for t, h in zip(leg.get_texts(), leg.legend_handles)]
print(pairs)
for (text, color), line in zip(pairs, ax.get_lines()):
    xd, yd = line.get_data()
    ax.annotate(text, (xd[-1], yd[-1]), xytext=(4, 0), textcoords="offset points", color=color, va="center")
leg.remove()
print(ax.get_legend(), len(ax.texts))
fig.savefig("inline_labels.png", dpi=50)
