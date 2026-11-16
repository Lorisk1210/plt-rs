# Tick labels on the right and top as well (label both sides) and moving the y label to the right.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5.5, 3.5))
ax.plot(np.arange(20), np.arange(20) ** 1.2)
ax.yaxis.tick_right()
ax.yaxis.set_label_position("right")
ax.set_ylabel("value (right)")
ax.tick_params(labeltop=True, top=True)
ax.xaxis.set_ticks_position("both")
ax.set_xlabel("index")
print(ax.yaxis.get_ticks_position(), ax.xaxis.get_ticks_position())
fig.savefig("both_sides.png")
