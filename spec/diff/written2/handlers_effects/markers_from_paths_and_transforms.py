# MarkerStyle built from Path objects, vertices lists and mathtext, with transform, cap/join styles; compare their geometry.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.markers import MarkerStyle
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

arrow = Path([(0, -1), (0.4, 0), (0.15, 0), (0.15, 1), (-0.15, 1), (-0.15, 0), (-0.4, 0), (0, -1)], closed=True)
specs = {
    "path": MarkerStyle(arrow),
    "verts": MarkerStyle([(-1, -1), (1, -1), (0, 1)]),
    "math": MarkerStyle(r"$\heartsuit$"),
    "rotated": MarkerStyle("D", transform=Affine2D().rotate_deg(20).scale(1, 0.5)),
    "capped": MarkerStyle("1", capstyle="round", joinstyle="bevel"),
    "half": MarkerStyle("o", fillstyle="top"),
}
for name, m in specs.items():
    p = m.get_path()
    t = m.get_transform()
    print(name, len(p.vertices), np.round(t.get_matrix(), 4).tolist()[0], m.is_filled(), m.get_fillstyle(),
          m.get_capstyle(), m.get_joinstyle())
fig, ax = plt.subplots(figsize=(6, 2))
for k, (name, m) in enumerate(specs.items()):
    ax.plot(k, 0, marker=m, ms=20, mfc="C1", mfcalt="C2", ls="")
ax.set_xlim(-1, len(specs))
fig.savefig("markers.png")
alt = specs["half"].get_alt_path()
print(alt is not None, np.round(specs["half"].get_alt_transform().get_matrix(), 4).tolist()[0])
print(MarkerStyle("o").scaled(2, 1).get_transform().get_matrix().round(4).tolist()[0])
print(MarkerStyle("^").rotated(deg=90).get_transform().get_matrix().round(4).tolist()[0])
