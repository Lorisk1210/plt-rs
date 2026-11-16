# Log axis with labelled minor ticks (LogFormatterSciNotation, LogLocator subs) for a decay curve.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import LogLocator, LogFormatterSciNotation, NullFormatter

t = np.linspace(0, 10, 100)
activity = 5000 * np.exp(-0.7 * t)
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(t, activity)
ax.set_yscale("log")
ax.yaxis.set_major_locator(LogLocator(base=10))
ax.yaxis.set_minor_locator(LogLocator(base=10, subs=(2, 5)))
ax.yaxis.set_major_formatter(LogFormatterSciNotation())
ax.yaxis.set_minor_formatter(NullFormatter())
ax.set_ylabel("counts per second")
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()])
fig.savefig("decay.png")
