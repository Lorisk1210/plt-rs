# Every Normalize subclass on the same data: values, inverse round trips, autoscale_None, scaled() and clipping.
import numpy as np
from matplotlib import colors

data = np.array([-10.0, -1, -0.1, 0, 0.1, 1, 10, 100])
norms = {
    "Normalize": colors.Normalize(-10, 100),
    "Normalize clip": colors.Normalize(-1, 1, clip=True),
    "LogNorm": colors.LogNorm(0.1, 100),
    "SymLogNorm": colors.SymLogNorm(linthresh=1, linscale=1, vmin=-10, vmax=100, base=10),
    "PowerNorm": colors.PowerNorm(0.5, vmin=0, vmax=100),
    "TwoSlopeNorm": colors.TwoSlopeNorm(vcenter=0, vmin=-10, vmax=100),
    "CenteredNorm": colors.CenteredNorm(vcenter=1, halfrange=5),
    "AsinhNorm": colors.AsinhNorm(linear_width=1, vmin=-10, vmax=100),
    "BoundaryNorm": colors.BoundaryNorm([-10, 0, 1, 10, 100], ncolors=256),
    "NoNorm": colors.NoNorm(),
    "FuncNorm": colors.FuncNorm((np.cbrt, lambda x: x ** 3), vmin=-10, vmax=100),
}
for name, n in norms.items():
    with np.errstate(all="ignore"):
        out = n(data)
    vals = np.ma.filled(np.ma.asarray(out).astype(float), np.nan)
    print(name, np.round(vals, 4).tolist())
    try:
        back = n.inverse(np.array([0.0, 0.25, 0.5, 1.0]))
        print(name, "inverse", np.round(np.ma.filled(np.ma.asarray(back).astype(float), np.nan), 4).tolist())
    except ValueError as e:
        print(name, "inverse ValueError", str(e)[:40])
n = colors.Normalize()
print(n.scaled(), n.vmin)
n.autoscale_None(np.array([3.0, 7.0]))
print(n.scaled(), n.vmin, n.vmax)
n.autoscale(np.array([1.0, 2.0]))
print(n.vmin, n.vmax)
try:
    colors.LogNorm(-1, 10)(np.array([1.0]))
except ValueError as e:
    print("ValueError", str(e)[:50])
