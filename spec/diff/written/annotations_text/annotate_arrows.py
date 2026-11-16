# Annotate the maximum and minimum of a curve with arrows of different styles.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 300)
y = np.sin(x) * np.exp(-x / 8)
imax, imin = y.argmax(), y.argmin()
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, y)
ax.annotate(f"max {y[imax]:.2f}", xy=(x[imax], y[imax]), xytext=(x[imax] + 1.5, y[imax] + 0.2),
            arrowprops=dict(facecolor="black", shrink=0.05, width=1, headwidth=6))
ax.annotate("min", xy=(x[imin], y[imin]), xytext=(x[imin] + 2, y[imin] - 0.3),
            arrowprops=dict(arrowstyle="->", connectionstyle="arc3,rad=-0.3", color="red"))
ax.annotate("decay", xy=(8, 0.0), xytext=(6, -0.8),
            arrowprops=dict(arrowstyle="fancy", fc="0.6", ec="none", connectionstyle="angle3,angleA=0,angleB=-90"))
ax.set_ylim(-1, 1.3)
print(round(x[imax], 4), round(y[imin], 4))
fig.savefig("extremes.png")
