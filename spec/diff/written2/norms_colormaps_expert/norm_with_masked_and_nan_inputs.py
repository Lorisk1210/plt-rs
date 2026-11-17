# Norms fed masked arrays, NaN, inf, empty arrays, scalars and integer arrays: outputs, masks and dtypes.
import numpy as np
from matplotlib import colors

inputs = {"masked": np.ma.masked_array([1.0, 2, 3], [0, 1, 0]), "nan": np.array([1.0, np.nan, 3]),
          "inf": np.array([1.0, np.inf, -np.inf]), "empty": np.array([]), "scalar": 2.0, "ints": np.array([1, 2, 3]),
          "bool": np.array([True, False]), "float32": np.array([1, 2, 3], np.float32), "list": [1, 2, 3]}
for name, data in inputs.items():
    for norm in (colors.Normalize(1, 3), colors.LogNorm(1, 3), colors.PowerNorm(2, 1, 3)):
        try:
            with np.errstate(all="ignore"):
                out = norm(data)
            if np.ndim(out) == 0:
                print(name, type(norm).__name__, type(out).__name__, round(float(out), 4))
            else:
                print(name, type(norm).__name__, np.asarray(out).dtype, np.ma.getmaskarray(out).tolist(),
                      np.round(np.ma.filled(out.astype(float), -9), 4).tolist())
        except Exception as e:
            print(name, type(norm).__name__, type(e).__name__)
n = colors.Normalize()
with np.errstate(all="ignore"):
    print(n(np.array([np.nan, np.nan])).tolist(), n.vmin, n.vmax)
n2 = colors.Normalize()
n2(np.ma.masked_all(3))
print(n2.vmin, n2.vmax)
print(colors.Normalize(5, 5)(np.array([4.0, 5, 6])).tolist())
tn = colors.TwoSlopeNorm(0)
with np.errstate(all="ignore"):
    print(np.round(tn(np.array([-2.0, 0, 4])), 4).tolist(), tn.vmin, tn.vmax)
sn = colors.SymLogNorm(1)
print(np.round(sn(np.array([-10.0, 0, 10])), 4).tolist())
print(np.round(colors.Normalize(0, 10)(np.int64(5)), 4))
