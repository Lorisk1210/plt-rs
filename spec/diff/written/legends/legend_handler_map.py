# Legend with handler_map: tuple handles (line over band) and HandlerLine2D numpoints override.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.legend_handler import HandlerLine2D, HandlerTuple

x = np.linspace(0, 10, 50)
mean = np.sin(x)
fig, ax = plt.subplots(figsize=(6, 4))
band = ax.fill_between(x, mean - 0.3, mean + 0.3, color="C0", alpha=0.25)
line, = ax.plot(x, mean, color="C0")
dots, = ax.plot(x[::5], mean[::5] + 0.6, "ko", ms=4)
ax.legend([(band, line), dots], ["mean ± sd", "samples"],
          handler_map={tuple: HandlerTuple(ndivide=None), dots: HandlerLine2D(numpoints=3)})
fig.savefig("handler_map.png")
print(type(ax.get_legend().legend_handles[0]).__name__)
