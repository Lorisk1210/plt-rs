# FloatingSubplot with a polar GridHelperCurveLinear: extremes, grid finders and an auxiliary transform for plotting.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D
import mpl_toolkits.axisartist.floating_axes as floating_axes
import mpl_toolkits.axisartist.angle_helper as angle_helper
from mpl_toolkits.axisartist.grid_finder import MaxNLocator, FixedLocator, DictFormatter

tr = Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform(apply_theta_transforms=False)
grid_locator1 = angle_helper.LocatorDMS(4)
tick_formatter1 = angle_helper.FormatterDMS()
grid_helper = floating_axes.GridHelperCurveLinear(tr, extremes=(0, 90, 1, 5), grid_locator1=grid_locator1,
                                                  grid_locator2=MaxNLocator(4), tick_formatter1=tick_formatter1)
fig = plt.figure(figsize=(5, 5), dpi=80)
ax = fig.add_subplot(111, axes_class=floating_axes.FloatingAxes, grid_helper=grid_helper)
aux = ax.get_aux_axes(tr)
theta = np.linspace(5, 85, 20)
aux.plot(theta, 1 + 3 * theta / 90, "o-")
ax.axis["left"].label.set_text("radius")
ax.axis["bottom"].label.set_text("angle")
ax.grid(True)
fig.canvas.draw()
print(sorted(ax.axis.keys()), type(aux).__name__)
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
print(grid_locator1(0, 90), MaxNLocator(4)(1, 5))
print(tick_formatter1("left", 1, [0, 30, 60]))
print([round(v, 2) for v in ax.get_window_extent().bounds])
print(ax.axis["left"].label.get_text(), ax.axis["bottom"].major_ticklabels.get_visible())
fixed = floating_axes.GridHelperCurveLinear(tr, extremes=(0, 60, 0, 2), grid_locator1=FixedLocator([0, 20, 40, 60]),
                                            tick_formatter1=DictFormatter({0: "zero", 20: "20", 40: "40", 60: "sixty"}))
ax2 = fig.add_axes([0.6, 0.6, 0.35, 0.35], axes_class=floating_axes.FloatingAxes, grid_helper=fixed)
fig.canvas.draw()
print(fixed.grid_finder.tick_formatter1("bottom", 1, [0, 20, 40, 60]))
fig.savefig("floating.png")
