# host_subplot with parasite twins (twinx, twin with a transform) and an offset third axis; limits and labels of each.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import host_subplot
from matplotlib.transforms import Affine2D

fig = plt.figure(figsize=(6, 4))
host = host_subplot(111, figure=fig)
par1 = host.twinx()
par2 = host.twinx()
par3 = host.twin(Affine2D().scale(2, 1))
x = np.linspace(0, 10, 30)
p0, = host.plot(x, x, label="host")
p1, = par1.plot(x, x ** 2, label="square")
p2, = par2.plot(x, np.sqrt(x), label="root")
host.set_xlabel("x")
host.set_ylabel("linear")
par1.set_ylabel("square")
par2.set_ylabel("root")
fig.canvas.draw()
print(type(host).__name__, [type(p).__name__ for p in host.parasites])
print([round(v, 2) for v in host.get_ylim()], [round(v, 2) for v in par1.get_ylim()], [round(v, 2) for v in par2.get_ylim()])
print([round(v, 2) for v in par3.get_xlim()], [round(v, 2) for v in host.get_xlim()])
host.legend(handles=[p0, p1, p2])
fig.canvas.draw()
print([t.get_text() for t in host.get_legend().get_texts()])
print(len(host.get_children()) > 5, host.get_tightbbox(fig.canvas.get_renderer()).width > 0)
par1.remove()
fig.canvas.draw()
print([type(p).__name__ for p in host.parasites], len(fig.axes))
fig.savefig("host.png")
