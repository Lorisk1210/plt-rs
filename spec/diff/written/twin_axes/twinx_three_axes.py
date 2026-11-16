# Three y axes on one plot: a second twinx with its spine moved outward ("parasite" style by hand).
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 50)
fig, host = plt.subplots(figsize=(7, 4))
fig.subplots_adjust(right=0.75)
par1 = host.twinx()
par2 = host.twinx()
par2.spines["right"].set_position(("axes", 1.2))
p1, = host.plot(x, x, "b-", label="distance")
p2, = par1.plot(x, np.sqrt(x) * 10, "r-", label="speed")
p3, = par2.plot(x, np.log1p(x), "g-", label="fuel")
host.set_xlabel("time")
host.set_ylabel("distance")
par1.set_ylabel("speed")
par2.set_ylabel("fuel")
for ax, p in [(host, p1), (par1, p2), (par2, p3)]:
    ax.yaxis.label.set_color(p.get_color())
host.legend(handles=[p1, p2, p3], loc="lower right")
print(par2.spines["right"].get_position())
fig.savefig("three_axes.png")
