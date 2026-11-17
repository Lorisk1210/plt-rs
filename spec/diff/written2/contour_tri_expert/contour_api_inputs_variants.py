# contour/contourf call signatures: Z only, X/Y vectors, 2-D X/Y, origin/extent for images, transposed shapes and errors.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

Z = np.add.outer(np.arange(5), np.arange(7) ** 0.5)
fig, ax = plt.subplots()
variants = [
    ("Z", lambda: ax.contour(Z)),
    ("Z,n", lambda: ax.contour(Z, 3)),
    ("vec", lambda: ax.contour(np.arange(7), np.arange(5), Z)),
    ("mesh", lambda: ax.contour(*np.meshgrid(np.arange(7), np.arange(5)), Z)),
    ("origin lower", lambda: ax.contour(Z, origin="lower", extent=(0, 1, 0, 1))),
    ("origin upper", lambda: ax.contour(Z, origin="upper", extent=(0, 1, 0, 1))),
    ("origin image", lambda: ax.contour(Z, origin="image")),
    ("bad shape", lambda: ax.contour(np.arange(5), np.arange(7), Z)),
    ("1d Z", lambda: ax.contour(np.arange(5))),
    ("masked all", lambda: ax.contour(np.ma.masked_all((4, 4)))),
    ("nan", lambda: ax.contourf(np.where(Z > 4, np.nan, Z))),
    ("list levels", lambda: ax.contourf(Z, levels=[1, 3, 5, 7])),
    ("unsorted", lambda: ax.contour(Z, levels=[3, 1])),
]
for name, fn in variants:
    try:
        cs = fn()
        ext = cs.get_paths()[0].get_extents() if cs.get_paths() and len(cs.get_paths()[0].vertices) else None
        print(name, np.round(cs.levels, 3).tolist(), len(cs.get_paths()), None if ext is None else [round(v, 3) for v in ext.bounds])
    except Exception as e:
        print(name, type(e).__name__, str(e)[:60])
fig.savefig("inputs.png")
