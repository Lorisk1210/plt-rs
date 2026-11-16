# Power ratios plotted in decibels on a secondary axis computed with 10*log10.
import numpy as np
import matplotlib.pyplot as plt

ratio = np.logspace(-3, 3, 50)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.semilogx(ratio, ratio ** 0.5)
ax.set_xlabel("power ratio")
sec = ax.secondary_xaxis("top", functions=(lambda r: 10 * np.log10(np.clip(r, 1e-12, None)), lambda db: 10 ** (db / 10)))
sec.set_xlabel("dB")
fig.canvas.draw()
print(ax.get_xscale(), [round(v, 4) for v in ax.get_xlim()])
fig.savefig("decibel.png")
