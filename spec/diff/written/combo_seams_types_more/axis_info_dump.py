# Dump axis internals users read: locator/formatter classes, tick counts, label positions, offset text, scale names.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(12, 3.5))
axs[0].plot(np.arange(100) * 1e5)
axs[1].semilogy(np.logspace(0, 5, 10))
axs[2].plot(["a", "b", "c"], [1, 2, 3])
fig.canvas.draw()
for ax in axs:
    for axis in (ax.xaxis, ax.yaxis):
        print(axis.axis_name, type(axis.get_major_locator()).__name__, type(axis.get_major_formatter()).__name__,
              len(axis.get_major_ticks()), axis.get_label_position(), repr(axis.get_offset_text().get_text()),
              axis.get_scale())
fig.savefig("axis_info.png", dpi=40)
