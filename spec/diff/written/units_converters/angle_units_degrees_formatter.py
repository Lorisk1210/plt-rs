# Degree-sign tick labels on an angle axis and radian data converted with np.rad2deg.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import MultipleLocator, FuncFormatter

theta = np.linspace(0, np.pi, 100)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(np.rad2deg(theta), np.sin(theta) ** 2)
ax.xaxis.set_major_locator(MultipleLocator(30))
ax.xaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{v:.0f}°"))
ax.set_xlim(0, 180)
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("degrees.png")
