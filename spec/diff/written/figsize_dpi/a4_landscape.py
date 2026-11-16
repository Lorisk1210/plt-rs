# A4 landscape figure for printing, sized in centimetres, with the size checked in pixels at 150 dpi.
import numpy as np
import matplotlib.pyplot as plt

cm = 1 / 2.54
fig, ax = plt.subplots(figsize=(29.7 * cm, 21.0 * cm), dpi=150)
x = np.linspace(0, 4 * np.pi, 400)
ax.plot(x, np.sin(x), x, np.sin(2 * x), x, np.sin(3 * x))
ax.set_title("Harmonics")
print(fig.get_size_inches())
print(fig.dpi)
print(fig.canvas.get_width_height())
fig.savefig("a4.png")
ax.legend(["1x", "2x", "3x"], loc="upper right")
ax.set_xlabel("phase [rad]")
fig.savefig("a4_with_legend.pdf")
