# Integer-only ticks for counts with MaxNLocator(integer=True) and at most 5 bins.
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

goals = [0, 2, 1, 3, 1, 0, 4, 2]
fig, ax = plt.subplots(figsize=(5, 3))
ax.bar(range(1, 9), goals, color="green")
ax.yaxis.set_major_locator(MaxNLocator(integer=True))
ax.xaxis.set_major_locator(MaxNLocator(nbins=5, integer=True))
ax.set_xlabel("match")
ax.set_ylabel("goals")
fig.canvas.draw()
print(ax.get_yticks(), ax.get_xticks())
ax.locator_params(axis="y", nbins=3)
print(ax.get_yticks())
fig.savefig("goals.png")
