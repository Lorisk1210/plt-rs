# The contour generator (contourpy) through matplotlib: ContourSet._contour_generator lines/filled for chosen levels.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x = np.linspace(-1, 1, 20)
X, Y = np.meshgrid(x, x)
Z = X ** 2 + Y ** 2
fig, ax = plt.subplots()
cs = ax.contour(X, Y, Z, levels=[0.25, 0.5])
gen = cs._contour_generator
print(type(gen).__name__)
lines = gen.create_contour(0.25)
print(type(lines).__name__, len(lines[0]) if isinstance(lines, tuple) else len(lines))
filled = gen.create_filled_contour(0.25, 0.5)
print(type(filled).__name__, len(filled))
for lev in (0.1, 0.5, 1.5, 3.0):
    segs = gen.create_contour(lev)
    pts = segs[0] if isinstance(segs, tuple) else segs
    n = sum(len(p) for p in pts)
    print(lev, n)
p = cs.get_paths()[0]
r = np.hypot(*p.vertices[p.codes != 79].T) if p.codes is not None else np.hypot(*p.vertices.T)
print(round(float(r.mean()), 4), round(float(r.std()), 4))
cf = ax.contourf(X, Y, Z, levels=[0, 0.25, 1])
p0 = cf.get_paths()[0]
print(len(p0.vertices), sorted(set(p0.codes.tolist())))
print(cs.get_transform() is ax.transData, cf.get_transform() is ax.transData)
fig.savefig("generator.png")
