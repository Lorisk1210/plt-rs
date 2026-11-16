# Emphasise the zero lines, soften the grid with alpha, and draw gridlines above bars (zorder).
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-5, 5, 11)
fig, ax = plt.subplots(figsize=(5.5, 3.5))
ax.bar(x, x ** 2 - 10, color="lightseagreen")
ax.axhline(0, color="black", lw=1.2, zorder=4)
ax.axvline(0, color="black", lw=1.2, zorder=4)
ax.grid(True, alpha=0.5, zorder=5)
ax.set_axisbelow(False)
ax.set_title("grid on top")
print(ax.get_axisbelow())
fig.savefig("zero_lines.png")
