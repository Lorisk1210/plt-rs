# Use the Agg canvas without pyplot (web server style): Figure + FigureCanvasAgg + print_png.
import io
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

fig = Figure(figsize=(4, 3), dpi=72)
canvas = FigureCanvasAgg(fig)
ax = fig.add_subplot()
ax.plot([1, 2, 3, 4], [2, 3, 5, 4], "o-")
ax.set_title("no pyplot")
out = io.BytesIO()
canvas.print_png(out)
print(out.getvalue()[:4], canvas.get_width_height())
canvas.draw()
renderer = canvas.get_renderer()
print(type(renderer).__name__, renderer.width, renderer.height)
fig.savefig("server.png")
