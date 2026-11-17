# matplotlib.backends.backend_template used as a backend: draws do nothing, savefig to its format, and its classes inspected.
import io
import matplotlib
matplotlib.use("template")
import matplotlib.pyplot as plt
from matplotlib.backends import backend_template as bt

print(matplotlib.get_backend())
fig, ax = plt.subplots()
ax.plot([1, 2, 3])
ax.set_title("template")
print(type(fig.canvas).__name__, type(fig.canvas.manager).__name__)
fig.canvas.draw()
print(type(fig.canvas).draw is matplotlib.backend_bases.FigureCanvasBase.draw)
print(sorted(fig.canvas.get_supported_filetypes())[:4], fig.canvas.get_default_filetype())
buf = io.BytesIO()
fig.savefig(buf, format="foo")
print(len(buf.getvalue()))
r = bt.RendererTemplate(72)
print(r.get_canvas_width_height(), r.flipy(), r.points_to_pixels(10), type(r.new_gc()).__name__)
w, h, d = r.get_text_width_height_descent("abc", matplotlib.font_manager.FontProperties(), False)
print(w, h, d)
print(issubclass(bt.FigureCanvasTemplate, matplotlib.backend_bases.FigureCanvasBase), bt.FigureCanvas is bt.FigureCanvasTemplate)
plt.close("all")
matplotlib.use("agg")
fig2, ax2 = plt.subplots()
print(type(fig2.canvas).__name__)
fig2.savefig("after_template.png")
print(matplotlib.get_backend(), plt.get_backend())
print(bt.FigureManager is bt.FigureManagerTemplate, bt.GraphicsContextTemplate.__mro__[1].__name__)
