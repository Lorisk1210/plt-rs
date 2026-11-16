# Annotation-heavy chart: arrows with different connection styles, boxed texts, spans with labels and a callout inset.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(100)
y = np.cumsum(np.sin(x / 7) + 0.2)
fig, ax = plt.subplots(figsize=(10, 5))
ax.plot(x, y, color="k")
ax.axvspan(20, 35, color="C0", alpha=0.15)
ax.text(27.5, ax.get_ylim()[1] * 0.95, "phase 1", ha="center")
ax.axvspan(60, 80, color="C1", alpha=0.15)
ax.text(70, ax.get_ylim()[1] * 0.95, "phase 2", ha="center")
for xi, style, conn in [(10, "->", "arc3,rad=0.3"), (45, "-|>", "angle3,angleA=90,angleB=0"), (90, "fancy", "arc3,rad=-0.2")]:
    ax.annotate(f"t={xi}", xy=(xi, y[xi]), xytext=(xi + 5, y[xi] + 6),
                arrowprops=dict(arrowstyle=style, connectionstyle=conn, fc="0.5"),
                bbox=dict(boxstyle="round", fc="lightyellow"))
ins = ax.inset_axes([0.05, 0.55, 0.25, 0.35])
ins.plot(x[40:50], y[40:50], "r.-")
ax.indicate_inset_zoom(ins, edgecolor="r")
print(len(ax.texts), len(ax.patches))
fig.savefig("annotated.png", dpi=50)
