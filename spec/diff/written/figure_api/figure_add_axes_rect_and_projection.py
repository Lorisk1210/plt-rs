# fig.add_axes with rect and projection keywords, label, and axes found again with fig.axes.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(7, 3.5))
cart = fig.add_axes((0.07, 0.15, 0.4, 0.75), label="cartesian")
polar = fig.add_axes((0.55, 0.1, 0.4, 0.8), projection="polar", label="polar")
cart.plot(np.linspace(0, 1, 10), np.linspace(0, 1, 10) ** 2)
polar.plot(np.linspace(0, 2 * np.pi, 50), np.ones(50))
print([(a.get_label(), a.name) for a in fig.axes])
fig.savefig("rects.png")
fig.savefig("figure_add_axes_rect_and_projection_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
