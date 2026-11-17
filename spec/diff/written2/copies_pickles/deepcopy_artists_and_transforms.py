# copy.copy and copy.deepcopy of artists, paths, transforms, colormaps, norms and FontProperties: independence of the copies.
import copy
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Affine2D, Bbox
from matplotlib.path import Path
from matplotlib.font_manager import FontProperties
from matplotlib import colors

t = Affine2D().rotate_deg(30)
t2 = copy.deepcopy(t)
t2.translate(5, 0)
print(np.round(t.get_matrix(), 3).tolist() == np.round(Affine2D().rotate_deg(30).get_matrix(), 3).tolist(),
      np.round(t2.get_matrix()[0], 3).tolist())
b = Bbox([[0, 0], [1, 1]])
b2 = copy.copy(b)
b2.x1 = 5
print(b.x1, b2.x1)
p = Path([(0, 0), (1, 1)])
p2 = copy.deepcopy(p)
p2.vertices[0] = (9, 9)
print(p.vertices[0].tolist(), p2.vertices[0].tolist())
cmap = plt.get_cmap("viridis")
c2 = copy.copy(cmap)
c2.set_bad("red")
print(cmap.get_bad().tolist(), c2.get_bad().tolist(), c2.name)
norm = colors.Normalize(0, 1)
n2 = copy.deepcopy(norm)
n2.vmax = 10
print(norm.vmax, n2.vmax)
fp = FontProperties(size=10)
fp2 = copy.copy(fp)
fp2.set_size(20)
print(fp.get_size(), fp2.get_size())
fig, ax = plt.subplots()
ln, = ax.plot([0, 1], [0, 1])
ln2 = copy.copy(ln)
ln2.set_color("r")
print(ln.get_color(), ln2.get_color(), ln2.axes is ax, ln2 in ax.lines)
try:
    ln3 = copy.deepcopy(ln)
    print(type(ln3).__name__, ln3.axes is ax, ln3.get_figure() is fig)
except Exception as e:
    print(type(e).__name__, str(e)[:60])
