# Arrays shared or copied by setters and getters: set_data with a reused array, get_offsets mutation, Path vertices from arrays.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.path import Path

fig, ax = plt.subplots()
x = np.linspace(0, 1, 5)
y = x ** 2
ln, = ax.plot(x, y)
y[0] = 99
print(ln.get_ydata()[0], ln.get_ydata(orig=True)[0])
fig.canvas.draw()
print(ln.get_ydata()[0], ln.get_xydata()[0].tolist())
offsets = np.array([[0, 0], [1, 1]], float)
sc = ax.scatter(offsets[:, 0], offsets[:, 1])
got = sc.get_offsets()
got[0, 0] = 5
print(sc.get_offsets()[0].tolist())
sc.set_offsets(offsets)
offsets[1, 1] = 7
print(sc.get_offsets()[1].tolist())
verts = np.array([[0, 0], [1, 0], [1, 1]], float)
p = Path(verts)
verts[0, 0] = 3
print(p.vertices[0].tolist(), p.vertices is verts)
p2 = Path(verts, readonly=True)
print(p2.vertices.flags.writeable, p.vertices.flags.writeable)
data = np.arange(9.0).reshape(3, 3)
im = ax.imshow(data)
data[0, 0] = 100
print(im.get_array()[0, 0], im.get_array() is data)
xs = ln.get_xdata()
xs[1] = -1
print(ln.get_xdata()[1])
ln.set_xdata(np.array([5, 6, 7, 8, 9]))
fig.canvas.draw()
print(ln.get_xdata().tolist(), [round(v, 2) for v in ln.get_path().vertices[:, 0]])
fig.savefig("shared.png")
