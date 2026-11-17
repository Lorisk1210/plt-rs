# Legends of composite handles: tuples with HandlerTuple(ndivide), errorbar and stem containers, and update_func on HandlerLine2D.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.legend_handler import HandlerTuple, HandlerLine2D, HandlerErrorbar


def thicker(legend_handle, orig_handle):
    legend_handle.update_from(orig_handle)
    legend_handle.set_linewidth(4)


fig, ax = plt.subplots(figsize=(5, 3.5))
x = np.arange(5)
l1, = ax.plot(x, x, "o-", color="C0")
l2, = ax.plot(x, x + 1, "s--", color="C1")
eb = ax.errorbar(x, x + 2, yerr=0.3, xerr=0.1, fmt="^", color="C2")
st = ax.stem(x, x + 3, linefmt="C3-", markerfmt="C3o", basefmt="k:")
leg = ax.legend(
    [(l1, l2), eb, st, l1],
    ["pair", "errorbar", "stem", "thick"],
    handler_map={
        tuple: HandlerTuple(ndivide=None, pad=0.5),
        l1: HandlerLine2D(update_func=thicker),
        type(eb): HandlerErrorbar(xerr_size=0.8),
    },
    handlelength=4,
)
fig.canvas.draw()
for h in leg.legend_handles:
    print(type(h).__name__, getattr(h, "get_linewidth", lambda: None)())
print(len(leg.get_lines()), len(leg.get_texts()))
print([t.get_text() for t in leg.get_texts()])
ext = leg.get_window_extent()
print([round(v, 1) for v in ext.bounds])
leg2 = ax.legend([(l1, l2)], ["split"], handler_map={tuple: HandlerTuple(ndivide=2, pad=0.0)}, loc="lower right")
fig.canvas.draw()
print(type(leg2.legend_handles[0]).__name__, leg2._loc)
fig.savefig("tuple_legend.png")
