# fig.bbox, bbox_inches, transFigure and canvas width/height as figsize and dpi change, and get_size_inches round trips.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(4, 3), dpi=100)
def state(tag):
    print(tag, fig.bbox.bounds, fig.bbox_inches.bounds, fig.canvas.get_width_height(), tuple(fig.get_size_inches()),
          fig.get_dpi())


state("start")
fig.set_size_inches(5.5, 2.25)
state("resize")
fig.set_dpi(72)
state("dpi")
fig.set_figwidth(3.3)
fig.set_figheight(1.1)
state("w/h")
fig.set_size_inches((6, 4), forward=False)
state("tuple")
fig.dpi = 150
state("attr")
print(fig.canvas.get_width_height(physical=True), fig.canvas.device_pixel_ratio)
fig.canvas._set_device_pixel_ratio(2)
state("dpr")
print(fig.canvas.get_width_height(physical=True), fig._original_dpi)
fig.canvas.draw()
print(fig.canvas.get_renderer().width, fig.canvas.get_renderer().height)
try:
    fig.set_size_inches(-1, 2)
except ValueError as e:
    print("ValueError", str(e)[:60])
