# Fill between two curves where one is above the other, with interpolate=True at crossings.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 4 * np.pi, 200)
supply = np.sin(x) + 1.2
demand = 0.8 * np.cos(x / 2) + 1.3
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(x, supply, label="supply")
ax.plot(x, demand, label="demand")
ax.fill_between(x, supply, demand, where=supply >= demand, interpolate=True, color="green", alpha=0.3, label="surplus")
ax.fill_between(x, supply, demand, where=supply < demand, interpolate=True, color="red", alpha=0.3, label="deficit")
ax.legend(ncols=2)
print(len(ax.collections))
fig.savefig("surplus.png")
