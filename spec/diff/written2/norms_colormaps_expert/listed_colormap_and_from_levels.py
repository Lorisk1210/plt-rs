# ListedColormap from lists/arrays, from_levels_and_colors with extends, BoundaryNorm with extend and clip; colours per value.
import numpy as np
from matplotlib import colors

lc = colors.ListedColormap(["#000000", "red", (0, 1, 0), (0, 0, 1, 0.5)], name="four")
print(lc.N, lc.name, np.round(lc(np.arange(4)), 3).tolist())
print(np.round(lc(np.linspace(0, 1, 6)), 3).tolist())
lc2 = colors.ListedColormap(np.linspace(0, 1, 9).reshape(3, 3))
print(lc2.N, np.round(lc2(2), 3).tolist())
for extend in ("neither", "min", "max", "both"):
    ncol = {"neither": 3, "min": 4, "max": 4, "both": 5}[extend]
    cmap, norm = colors.from_levels_and_colors([0, 1, 2, 3], [f"C{k}" for k in range(ncol)], extend=extend)
    vals = np.array([-1, 0.5, 1.5, 2.5, 4])
    print(extend, cmap.N, norm.extend, norm(vals).tolist(), [colors.to_hex(c) for c in cmap(norm(vals))])
try:
    colors.from_levels_and_colors([0, 1, 2], ["r", "g", "b"])
except ValueError as e:
    print("ValueError", str(e)[:60])
bn = colors.BoundaryNorm([0, 2, 4, 6], ncolors=10, clip=True)
print(bn(np.array([-1, 1, 3, 5, 7])).tolist(), bn.Ncmap, bn.N)
bn2 = colors.BoundaryNorm([0, 2, 4, 6], ncolors=10, extend="both")
print(bn2(np.array([-1, 1, 3, 5, 7])).tolist())
try:
    colors.BoundaryNorm([0, 1], ncolors=10).inverse(0.5)
except ValueError as e:
    print("ValueError", str(e)[:60])
print(np.round(colors.ListedColormap(["r", "g"]).resampled(4)(np.arange(4)), 3).tolist())
print(colors.ListedColormap(["r", "g"]).colors, colors.ListedColormap(["r", "g", "b"]).monochrome)
cm3, nm3 = colors.from_levels_and_colors([0, 10, 20], ["r", "b"])
print(cm3.N, nm3.boundaries.tolist(), [colors.to_hex(cm3(nm3(v))) for v in (5, 15)])
