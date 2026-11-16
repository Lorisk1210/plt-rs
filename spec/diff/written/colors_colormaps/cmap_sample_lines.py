# Colour a family of lines by sampling a colormap with np.linspace (a very common idiom).
import numpy as np
import matplotlib.pyplot as plt

temps = np.arange(250, 451, 25)
colors = plt.cm.coolwarm(np.linspace(0, 1, len(temps)))
x = np.linspace(0.1, 5, 200)
fig, ax = plt.subplots(figsize=(6, 4))
for T, c in zip(temps, colors):
    ax.plot(x, x ** 3 / (np.exp(x * 300 / T) - 1), color=c, label=f"{T} K")
ax.legend(ncols=3, fontsize=7)
ax.set_xlabel("energy")
print(np.round(colors[0], 4), np.round(colors[-1], 4))
fig.savefig("planck.png")
