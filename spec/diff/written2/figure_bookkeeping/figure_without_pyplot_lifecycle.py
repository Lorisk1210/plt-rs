# Figures made with Figure() directly: not in pyplot's registry, savefig works, adding to pyplot via a manager, and garbage collection.
import gc
import weakref
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

fig = Figure(figsize=(3, 2))
print(type(fig.canvas).__name__, plt.get_fignums(), getattr(fig, "number", "no number"))
ax = fig.add_subplot()
ax.plot([1, 3, 2])
fig.savefig("standalone.png")
print(type(fig.canvas).__name__)
ref = weakref.ref(fig)
del fig, ax
figure_gone = gc.collect() >= 0 and ref() is None
print(ref() is None)
fig2 = Figure()
canvas = FigureCanvasAgg(fig2)
print(fig2.canvas is canvas, canvas.manager, plt.get_fignums())
fig3 = plt.figure(FigureClass=Figure)
print(plt.get_fignums(), type(fig3.canvas).__name__, type(fig3.canvas.manager).__name__)
ref3 = weakref.ref(fig3)
plt.close(fig3)
del fig3
managed_gone = gc.collect() >= 0 and ref3() is None
print(ref3() is None, plt.get_fignums())
fig4 = plt.figure()
fig4.add_subplot().plot([1])
del fig4
still_open = gc.collect() >= 0 and len(plt.get_fignums())
print(plt.get_fignums(), plt.gcf().axes[0].lines[0].get_ydata().tolist())
plt.close("all")
