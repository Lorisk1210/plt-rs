# axisartist floating axis: a curvilinear polar grid on rectangular axes (GridHelperCurveLinear).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D
import mpl_toolkits.axisartist.angle_helper as angle_helper
from mpl_toolkits.axisartist import HostAxes, GridHelperCurveLinear

tr = Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform(apply_theta_transforms=False)
extreme = angle_helper.ExtremeFinderCycle(20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf))
grid_helper = GridHelperCurveLinear(tr, extreme_finder=extreme, grid_locator1=angle_helper.LocatorDMS(12),
                                    tick_formatter1=angle_helper.FormatterDMS())
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(axes_class=HostAxes, grid_helper=grid_helper)
ax.axis["lat"] = ax.new_floating_axis(0, 60)
ax.axis["lat"].label.set_text("r = 60")
ax.set_aspect(1)
ax.set_xlim(-5, 12)
ax.set_ylim(-5, 10)
ax.grid(True, zorder=0)
fig.savefig("curvilinear.png")
print(sorted(ax.axis.keys()))
