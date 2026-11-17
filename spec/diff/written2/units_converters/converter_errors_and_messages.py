# What happens when units disagree: strings on a numeric axis, numbers on a category axis, dates after floats, broken converters.
import datetime
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.units as munits


def attempt(label, fn):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            fn()
            print(label, "ok", [str(w.message)[:50] for w in caught])
        except Exception as e:
            print(label, type(e).__name__, str(e).splitlines()[0][:70])


fig, ax = plt.subplots()
ax.plot([1, 2], [3, 4])
attempt("strings after numbers", lambda: ax.plot(["a", "b"], [1, 2]))
fig2, ax2 = plt.subplots()
ax2.bar(["x", "y"], [1, 2])
attempt("numbers after categories", lambda: ax2.plot([0.5, 1.5], [1, 1]))
attempt("mixed list", lambda: ax2.plot(["x", 3], [1, 1]))
fig3, ax3 = plt.subplots()
ax3.plot([1.0, 2.0], [0, 1])
attempt("dates after floats", lambda: (ax3.plot([datetime.date(2024, 1, 1)], [0]), fig3.canvas.draw()))


class Broken:
    pass


class BrokenConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        raise RuntimeError("cannot convert")


munits.registry[Broken] = BrokenConverter()
fig4, ax4 = plt.subplots()
attempt("broken converter", lambda: ax4.plot([Broken(), Broken()], [1, 2]))
attempt("set_xlim strings", lambda: ax.set_xlim("a", "b"))
attempt("datetime64 nat", lambda: plt.subplots()[1].plot(np.array(["NaT", "2024-01-01"], dtype="datetime64[D]"), [1, 2]))
del munits.registry[Broken]
attempt("object array", lambda: plt.subplots()[1].plot(np.array([object(), object()]), [1, 2]))
print(plt.get_fignums())
