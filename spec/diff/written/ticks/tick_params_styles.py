# Restyle ticks: inward, long, coloured, on all four sides, with label padding, "journal style".
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0.1, 5, 60)
fig, ax = plt.subplots(figsize=(5, 4))
ax.plot(x, np.log(x), "k")
ax.minorticks_on()
ax.tick_params(which="both", direction="in", top=True, right=True)
ax.tick_params(which="major", length=7, width=1.2, labelsize=11, pad=6)
ax.tick_params(which="minor", length=3.5, color="gray")
ax.tick_params(axis="y", labelcolor="navy")
ax.set_xlabel("x")
ax.set_ylabel("ln x")
print(ax.xaxis.get_major_ticks()[0].tick1line.get_markersize())
fig.savefig("journal.png")
