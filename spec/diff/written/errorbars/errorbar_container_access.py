# Access and restyle an ErrorbarContainer's parts after plotting, and remove it again.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(5)
fig, ax = plt.subplots(figsize=(5, 3.5))
eb = ax.errorbar(x, x ** 1.5, yerr=0.5 + x * 0.2, fmt="o-", capsize=5, label="growth")
data_line, caplines, barlinecols = eb.lines
data_line.set_markerfacecolor("white")
for cap in caplines:
    cap.set_markeredgewidth(2)
barlinecols[0].set_linestyle("--")
print(eb.has_xerr, eb.has_yerr, len(caplines))
fig.savefig("container.png")
eb2 = ax.errorbar(x, x, yerr=1, fmt="s")
eb2.remove()
print(len(ax.containers), len(ax.lines))
fig.savefig("container_removed.png")
