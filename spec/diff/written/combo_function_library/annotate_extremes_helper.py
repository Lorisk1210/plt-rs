# Helper annotating the extremes of every line in an axes by reading the lines back (get_xdata/get_ydata/get_color).
import numpy as np
import matplotlib.pyplot as plt


def annotate_extremes(ax, which="max"):
    notes = []
    for line in ax.get_lines():
        x, y = np.asarray(line.get_xdata()), np.asarray(line.get_ydata())
        i = int(np.argmax(y) if which == "max" else np.argmin(y))
        ax.annotate(f"{y[i]:.2f}", (x[i], y[i]), xytext=(0, 8 if which == "max" else -14),
                    textcoords="offset points", ha="center", color=line.get_color(), fontsize=8)
        notes.append((line.get_label(), round(float(x[i]), 3), round(float(y[i]), 3)))
    return notes


x = np.linspace(0, 6, 120)
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, np.sin(x), label="sin")
ax.plot(x, 0.8 * np.cos(1.3 * x), label="cos")
ax.plot(x, np.exp(-x / 2) * np.sin(3 * x), label="damped")
print(annotate_extremes(ax))
print(annotate_extremes(ax, "min"))
ax.legend()
print([t.get_color() for t in ax.texts][:3])
fig.savefig("extremes.png", dpi=50)
