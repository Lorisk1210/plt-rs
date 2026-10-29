"""Error bars: the activity of an enzyme and of a heat-stable variant
across temperatures, each point the mean of repeated measurements with its
standard deviation."""

import numpy as np

import pltrs.pyplot as plt

temperature = np.arange(20, 85, 5)
wild = 100 * np.exp(-(((temperature - 42) / 14) ** 2))
stable = 92 * np.exp(-(((temperature - 58) / 17) ** 2))

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
ax.errorbar(temperature, wild, yerr=4 + 0.06 * wild, fmt="o-", capsize=3, label="wild type")
ax.errorbar(temperature, stable, yerr=3 + 0.08 * stable, fmt="s--", capsize=3, label="heat-stable variant")
ax.set_title("Enzyme activity against temperature")
ax.set_xlabel("temperature (°C)")
ax.set_ylabel("activity (% of maximum)")
ax.set_ylim(0, 115)
ax.legend(loc="upper right")

fig.savefig("error_bars.png")
