# axes_grid1 host_subplot with parasite twin axes for three y scales (toolkit way).
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import host_subplot

host = host_subplot(111)
par = host.twinx()
host.set_xlabel("distance")
host.set_ylabel("density")
par.set_ylabel("temperature")
p1, = host.plot([0, 1, 2], [0, 1, 2], label="density")
p2, = par.plot([0, 1, 2], [0, 3, 2], label="temperature")
host.legend(labelcolor="linecolor")
host.yaxis.label.set_color(p1.get_color())
par.yaxis.label.set_color(p2.get_color())
plt.savefig("parasite.png")
print(type(host).__name__, type(par).__name__)
