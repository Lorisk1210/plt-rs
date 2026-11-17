# Call APIs matplotlib 3.11 deprecates and print the warning classes and messages (with versions masked to the major part).
import re
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import _api


def check(label, fn):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            out = fn()
        except Exception as e:
            print(label, "raised", type(e).__name__, str(e)[:50])
            return
    msgs = [(w.category.__name__, re.sub(r"\d+\.\d+", "X.Y", str(w.message))[:80]) for w in caught]
    print(label, msgs if msgs else "no warning")


fig, ax = plt.subplots()
tm = fig.canvas.new_timer()
check("timer remove args", lambda: (tm.add_callback(print, 1), tm.remove_callback(print, 1)))
from mpl_toolkits.axisartist import grid_finder as gf
from matplotlib.transforms import Affine2D
finder = gf.GridFinder(Affine2D(), grid_locator1=gf.MaxNLocator(3), grid_locator2=gf.MaxNLocator(3))
check("grid_info extents", lambda: finder.get_grid_info(0, 0, 1, 1))
check("polar transform", lambda: matplotlib.projections.PolarAxes.PolarTransform())
check("register cmap twice", lambda: matplotlib.colormaps.register(matplotlib.colormaps["viridis"], name="viridis"))
print(issubclass(_api.MatplotlibDeprecationWarning, DeprecationWarning), _api.MatplotlibDeprecationWarning.__module__)
print(matplotlib.MatplotlibDeprecationWarning is _api.MatplotlibDeprecationWarning)
with warnings.catch_warnings():
    warnings.simplefilter("error", matplotlib.MatplotlibDeprecationWarning)
    try:
        finder.get_grid_info(0, 0, 1, 1)
        print("no error")
    except matplotlib.MatplotlibDeprecationWarning as e:
        print("raised as error", type(e).__name__)
print(sorted(n for n in dir(_api) if "deprecat" in n.lower())[:5])
