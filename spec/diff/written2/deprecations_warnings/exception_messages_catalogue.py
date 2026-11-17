# Exceptions raised by bad arguments across the API: exception type and the start of each message, as error handling code sees them.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
cases = [
    ("plot mismatched", lambda: ax.plot([1, 2], [1, 2, 3])),
    ("bad color", lambda: ax.plot([1], color="nocolor")),
    ("bad linestyle", lambda: ax.plot([1], ls="~~")),
    ("bad marker", lambda: ax.plot([1], marker="?")),
    ("bad scale", lambda: ax.set_xscale("cubic")),
    ("bad loc", lambda: ax.legend(["a"], loc="middle")),
    ("bad cmap", lambda: ax.imshow(np.eye(2), cmap="nocmap")),
    ("imshow 1d", lambda: ax.imshow(np.arange(3))),
    ("subplot index", lambda: fig.add_subplot(2, 2, 5)),
    ("bad projection", lambda: fig.add_subplot(projection="sphere")),
    ("bar shape", lambda: ax.bar([1, 2], [1, 2, 3])),
    ("hist bins", lambda: ax.hist([1, 2], bins=-1)),
    ("set aspect", lambda: ax.set_aspect("wide")),
    ("bad kwarg", lambda: ax.plot([1], nonsense=3)),
    ("savefig format", lambda: fig.savefig("x.unknownfmt")),
    ("text bad ha", lambda: ax.text(0, 0, "t", ha="middle")),
    ("contour levels", lambda: ax.contour(np.eye(3), levels=[2, 1])),
    ("pcolormesh shape", lambda: ax.pcolormesh(np.zeros((2, 2)), np.zeros((3, 3)), np.zeros((4, 4)))),
    ("tick params", lambda: ax.tick_params(axis="z")),
    ("errorbar negative", lambda: ax.errorbar([1], [1], yerr=[-1])),
]
for label, fn in cases:
    try:
        fn()
        print(label, "no error")
    except Exception as e:
        print(label, type(e).__name__, str(e).splitlines()[0][:70] if str(e) else "")
