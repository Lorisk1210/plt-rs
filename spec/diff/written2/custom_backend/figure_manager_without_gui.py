# FigureManagerBase for headless canvases: new_figure_manager_given_figure, set_window_title, show warnings and pyplot.show on agg.
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import FigureManagerBase
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

fig, ax = plt.subplots()
mgr = fig.canvas.manager
print(type(mgr).__name__, mgr.num, mgr.get_window_title())
mgr.set_window_title("renamed")
print(mgr.get_window_title(), fig.get_label())
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    plt.show()
    fig.show()
print([str(w.message)[:70] for w in caught])
f2 = Figure()
c2 = FigureCanvasAgg(f2)
m2 = FigureCanvasAgg.new_manager(f2, 42)
print(type(m2).__name__, m2.num, f2.canvas.manager is m2, plt.fignum_exists(42))
m2.resize(300, 200)
m2.full_screen_toggle()
print(m2.toolbar, m2.toolmanager)
m3 = FigureManagerBase(FigureCanvasAgg(Figure()), 7)
print(m3.num, m3.get_window_title(), m3.canvas.figure is not None)
from matplotlib.backend_bases import NonGuiException
try:
    m3.show()
    print("shown")
except NonGuiException as e:
    print("NonGuiException", str(e)[:60])
print(matplotlib.rcParams["toolbar"], plt.isinteractive())
plt.close("all")
