# The axisartist AxisArtist parts: major_ticks, major_ticklabels, minor parts, label, invert_ticklabel_direction, set_ticklabel_direction.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist

fig = plt.figure(figsize=(5, 4), dpi=80)
ax = fig.add_subplot(axes_class=axisartist.Axes)
ax.set_xlim(0, 10)
ax.set_ylim(0, 1)
ax.minorticks_on()
bottom = ax.axis["bottom"]
left = ax.axis["left"]
bottom.major_ticks.set_ticksize(10)
bottom.minor_ticks.set_ticksize(4)
bottom.major_ticklabels.set_rotation(30)
bottom.major_ticklabels.set_color("purple")
left.invert_ticklabel_direction()
left.set_ticklabel_direction("+")
left.set_axislabel_direction("-")
left.label.set_text("value")
bottom.label.set_text("time")
bottom.label.set_pad(12)
fig.canvas.draw()
r = fig.canvas.get_renderer()
print(bottom.major_ticks.get_ticksize(), bottom.minor_ticks.get_ticksize(), bottom.major_ticklabels.get_rotation())
print(bottom.major_ticklabels.get_color(), left.major_ticklabels._axis_direction, left.label.get_rotation(), left.label.get_ha())
print([round(v, 1) for v in bottom.get_tightbbox(r).bounds], [round(v, 1) for v in left.get_tightbbox(r).bounds])
print(bottom.label.get_pad(), left.major_ticks.get_tick_out(), bottom.major_ticklabels.get_texts_widths_heights_descents(r)[:2])
ax.axis[:].major_ticks.set_color("r")
fig.canvas.draw()
print([ax.axis[k].major_ticks.get_color() for k in ("left", "bottom", "right", "top")])
ax.axis["top", "right"].set_visible(False)
print([ax.axis[k].get_visible() for k in ("left", "bottom", "right", "top")])
fig.savefig("axisartist_parts.png")
