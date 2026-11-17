# Contours of masked data with corner_mask on/off and each algorithm (mpl2005, mpl2014, serial, threaded); vertex counts compared.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x = np.linspace(0, 4, 30)
X, Y = np.meshgrid(x, x)
Z = np.sin(X) * np.cos(Y)
mask = (X - 2) ** 2 + (Y - 2) ** 2 < 0.8
mask[0, 5] = True
Zm = np.ma.masked_array(Z, mask)
fig, ax = plt.subplots(figsize=(3, 3))
for algorithm in ("mpl2005", "mpl2014", "serial", "threaded"):
    for corner in (True, False):
        try:
            cs = ax.contourf(X, Y, Zm, levels=5, algorithm=algorithm, corner_mask=corner)
            cl = ax.contour(X, Y, Zm, levels=5, algorithm=algorithm, corner_mask=corner)
            print(algorithm, corner, len(cs.get_paths()), sum(len(p.vertices) for p in cs.get_paths()),
                  sum(len(p.vertices) for p in cl.get_paths()))
            cs.remove()
            cl.remove()
        except ValueError as e:
            print(algorithm, corner, "ValueError", str(e)[:50])
with plt.rc_context({"contour.algorithm": "serial", "contour.corner_mask": False}):
    cs = ax.contour(X, Y, Zm)
    print(cs._algorithm, cs._corner_mask)
cs = ax.contour(X, Y, Z, levels=4, nchunk=4)
print(len(cs.get_paths()), cs.nchunk)
fig.savefig("algorithms.png")
