# axisartist host with parasite twinx axes offset by new_fixed_axis, the multi-y-axis pattern; offsets and label colours.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist
from mpl_toolkits.axes_grid1 import host_subplot

fig = plt.figure(figsize=(6, 4), dpi=80)
host = host_subplot(111, axes_class=axisartist.Axes, figure=fig)
plt.subplots_adjust(right=0.75)
par1 = host.twinx()
par2 = host.twinx()
par2.axis["right"] = par2.new_fixed_axis(loc="right", offset=(60, 0))
par1.axis["right"].toggle(all=True)
par2.axis["right"].toggle(all=True)
x = np.arange(5)
p0, = host.plot(x, x, label="Density")
p1, = par1.plot(x, x ** 2, label="Temperature")
p2, = par2.plot(x, np.sqrt(x), label="Velocity")
host.set(xlim=(0, 4), ylim=(0, 4), xlabel="Distance", ylabel="Density")
par1.set(ylim=(0, 16), ylabel="Temperature")
par2.set(ylim=(0, 2), ylabel="Velocity")
host.legend()
host.axis["left"].label.set_color(p0.get_color())
par1.axis["right"].label.set_color(p1.get_color())
par2.axis["right"].label.set_color(p2.get_color())
fig.canvas.draw()
r = fig.canvas.get_renderer()
for a in (host, par1, par2):
    print(type(a).__name__, sorted(a.axis.keys())[:4], [round(v, 2) for v in a.get_ylim()])
print([round(v, 1) for v in par2.axis["right"].get_tightbbox(r).bounds])
print([round(v, 1) for v in par1.axis["right"].get_tightbbox(r).bounds])
print(par2.axis["right"].label.get_color(), par2.axis["right"].label.get_text())
print([t.get_text() for t in host.get_legend().get_texts()])
fig.savefig("parasites.png")
