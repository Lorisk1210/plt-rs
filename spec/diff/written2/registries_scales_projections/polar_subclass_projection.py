# Subclass PolarAxes as a registered "compass" projection (north up, clockwise, degree labels) and read its state back.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.projections import register_projection
from matplotlib.projections.polar import PolarAxes


class CompassAxes(PolarAxes):
    name = "compass"

    def clear(self):
        super().clear()
        self.set_theta_zero_location("N")
        self.set_theta_direction(-1)
        self.set_thetagrids(range(0, 360, 45), ["N", "NE", "E", "SE", "S", "SW", "W", "NW"])
        self.set_rlabel_position(22.5)


register_projection(CompassAxes)
fig = plt.figure(figsize=(4, 4))
ax = fig.add_subplot(projection="compass")
dirs = np.deg2rad([0, 45, 200, 300])
ax.plot(dirs, [1, 2, 1.5, 2.5], "o-")
fig.canvas.draw()
print(type(ax).__name__, ax.name, round(ax.get_theta_offset(), 6), ax.get_theta_direction())
print([t.get_text() for t in ax.get_xticklabels()])
print(round(float(ax.get_rlabel_position()), 6), [round(v, 6) for v in ax.get_ylim()])
ax.cla()
print(round(ax.get_theta_offset(), 6), ax.get_theta_direction(), [t.get_text() for t in ax.get_xticklabels()][:3])
fig.savefig("compass.png")
fig2, ax2 = plt.subplots(subplot_kw=dict(projection="compass"))
print(type(ax2).__name__, ax2.get_theta_direction())
