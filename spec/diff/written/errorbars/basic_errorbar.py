# Measurements with symmetric y errors and asymmetric x errors, caps and a fitted line.
import numpy as np
import matplotlib.pyplot as plt

x = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
y = np.array([2.1, 3.9, 6.2, 7.8, 10.1, 12.2])
yerr = np.array([0.3, 0.4, 0.3, 0.5, 0.6, 0.4])
xerr = np.array([[0.1, 0.2, 0.1, 0.1, 0.2, 0.1], [0.2, 0.1, 0.3, 0.2, 0.1, 0.2]])
fig, ax = plt.subplots(figsize=(6, 4))
cont = ax.errorbar(x, y, yerr=yerr, xerr=xerr, fmt="o", capsize=4, ecolor="gray", elinewidth=1, label="data")
p = np.polyfit(x, y, 1, w=1 / yerr)
ax.plot(x, np.polyval(p, x), "r-", label=f"fit: {p[0]:.2f}x + {p[1]:.2f}")
ax.legend()
print(type(cont).__name__, len(cont.lines), np.round(p, 4))
fig.savefig("measurements.png")
