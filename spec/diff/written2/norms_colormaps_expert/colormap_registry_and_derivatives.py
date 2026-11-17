# The colormaps registry: register/unregister, copies on access, resampled/reversed/with_extremes, and bivariate/multivariate maps.
import numpy as np
import matplotlib
from matplotlib import colors

reg = matplotlib.colormaps
print(len(reg) > 150, "viridis" in reg, "viridis_r" in reg, sorted(reg)[:3])
v1, v2 = reg["viridis"], reg["viridis"]
print(v1 is v2, v1 == v2, v1.N)
mine = colors.LinearSegmentedColormap.from_list("written2_fire", ["black", "red", "yellow", "white"], N=64)
reg.register(mine)
print("written2_fire" in reg, reg["written2_fire"].N, reg["written2_fire"] is mine)
try:
    reg.register(mine)
except ValueError as e:
    print("ValueError", str(e)[:60])
reg.register(mine.resampled(8), name="written2_fire", force=True)
print(reg["written2_fire"].N)
reg.unregister("written2_fire")
print("written2_fire" in reg)
try:
    reg.unregister("viridis")
except ValueError as e:
    print("ValueError", str(e)[:60])
r = v1.reversed()
print(r.name, np.round(r(0.0), 4).tolist() == np.round(v1(1.0), 4).tolist())
ext = v1.with_extremes(bad="k", under="b", over="r")
print(ext.get_bad().tolist(), ext.get_under().tolist(), v1.get_under().tolist() != ext.get_under().tolist())
print(np.round(v1.resampled(3)(np.arange(3)), 4).tolist())
print(type(reg["tab10"]).__name__, reg["tab10"].N, type(reg["twilight"]).__name__)
print(np.round(reg["gray"]([0, 0.5, 1]), 4).tolist())
print(reg.get_cmap("magma").name, reg.get_cmap(v1) is v1)
seg = colors.LinearSegmentedColormap("seg", {"red": [(0, 0, 0), (1, 1, 1)], "green": [(0, 0, 0), (1, 0, 0)],
                                             "blue": [(0, 1, 1), (1, 0, 0)]}, N=5)
print(np.round(seg(np.arange(5)), 3).tolist())
print(hasattr(matplotlib, "bivar_colormaps"), hasattr(matplotlib, "multivar_colormaps"))
