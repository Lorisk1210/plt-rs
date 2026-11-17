# Sankey geometry options one at a time (trunklength, shoulder, margin, tolerance, head_angle) and the resulting patch extents.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.sankey import Sankey

flows = [1, -0.4, -0.3, -0.3]
orient = [0, 1, 0, -1]
base = dict(flows=flows, orientations=orient)
options = [dict(), dict(trunklength=2.0), dict(shoulder=0.1), dict(margin=0.8), dict(head_angle=60), dict(tolerance=0.5),
           dict(scale=0.5), dict(gap=0.5), dict(radius=0.2), dict(rotation=45)]
for opt in options:
    fig, ax = plt.subplots(figsize=(3, 3), dpi=40)
    init = {k: v for k, v in opt.items() if k in ("shoulder", "margin", "head_angle", "tolerance", "scale", "gap", "radius")}
    add = {k: v for k, v in opt.items() if k in ("trunklength", "rotation")}
    sk = Sankey(ax=ax, **init)
    sk.add(**base, **add)
    d = sk.finish()[0]
    ext = d.patch.get_path().get_extents()
    print(sorted(opt), len(d.patch.get_path().vertices), [round(v, 3) for v in ext.bounds])
    plt.close(fig)
fig, ax = plt.subplots()
import warnings
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    Sankey(ax=ax).add(flows=[1, -0.5]).finish()
print([str(w.message)[:60] for w in caught])
try:
    Sankey(ax=ax, gap=-1)
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    Sankey(ax=ax).add(flows=[1, -1], labels=["a", "b", "c"])
except ValueError as e:
    print("ValueError", str(e)[:60])
