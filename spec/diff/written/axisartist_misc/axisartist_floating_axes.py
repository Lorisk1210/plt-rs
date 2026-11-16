# axisartist: axes through the origin with SubplotZero and arrow-tipped axis lines.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axisartist.axislines import AxesZero

fig = plt.figure(figsize=(5, 4))
ax = fig.add_subplot(axes_class=AxesZero)
for direction in ["xzero", "yzero"]:
    ax.axis[direction].set_axisline_style("-|>")
    ax.axis[direction].set_visible(True)
for direction in ["left", "right", "bottom", "top"]:
    ax.axis[direction].set_visible(False)
x = np.linspace(-0.5, 1.0, 100)
ax.plot(x, np.sin(x * np.pi))
print(type(ax).__name__)
fig.savefig("axes_zero.png")
