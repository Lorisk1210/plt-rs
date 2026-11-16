# axisartist parasite axes with an offset third y axis (new_fixed_axis), toolkit-style.
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist
from mpl_toolkits.axes_grid1 import host_subplot

host = host_subplot(111, axes_class=axisartist.Axes)
plt.subplots_adjust(right=0.75)
par1 = host.twinx()
par2 = host.twinx()
par2.axis["right"] = par2.new_fixed_axis(loc="right", offset=(60, 0))
par1.axis["right"].toggle(all=True)
par2.axis["right"].toggle(all=True)
p1, = host.plot([0, 1, 2], [0, 1, 2], label="density")
p2, = par1.plot([0, 1, 2], [0, 3, 2], label="temperature")
p3, = par2.plot([0, 1, 2], [50, 30, 15], label="velocity")
host.set(xlim=(0, 2), ylim=(0, 2), xlabel="distance", ylabel="density")
par1.set(ylim=(0, 4), ylabel="temperature")
par2.set(ylim=(1, 65), ylabel="velocity")
host.legend()
host.axis["left"].label.set_color(p1.get_color())
plt.savefig("parasite_offset.png")
print(len(plt.gcf().axes))
