# QuadMesh from pcolormesh and TriMesh built by hand: coordinates, set_array shapes, antialiasing and get_datalim.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.tri as mtri
from matplotlib.collections import TriMesh

x = np.linspace(0, 1, 5)
X, Y = np.meshgrid(x, x ** 2)
fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3))
qm = a1.pcolormesh(X, Y, X * Y, shading="gouraud")
print(type(qm).__name__, qm.get_coordinates().shape, qm.get_array().shape, qm._shading)
qm2 = a1.pcolormesh(X, Y, (X * Y)[:-1, :-1], shading="flat", edgecolors="w", linewidth=0.5)
print(qm2.get_coordinates().shape, qm2.get_array().shape)
try:
    qm2.set_array(np.zeros((3, 3)))
except (ValueError, TypeError) as e:
    print(type(e).__name__, str(e)[:60])
qm2.set_array(np.ones((4, 4)) * 2)
print(qm2.get_array().shape, float(qm2.get_array().sum()))
tri = mtri.Triangulation(X.ravel(), Y.ravel())
tm = TriMesh(tri)
tm.set_array(np.hypot(X, Y).ravel())
a2.add_collection(tm)
a2.autoscale_view()
fig.canvas.draw()
print(type(tm).__name__, len(tri.triangles), tm.get_array().shape, [round(v, 3) for v in tm.get_datalim(a2.transData).bounds])
print(len(tm.get_paths()), len(tm.get_paths()[0].vertices))
print([round(v, 3) for v in qm.get_datalim(a1.transData).bounds], qm.get_antialiased())
fig.savefig("meshes.png")
