# grid_finder pieces on their own: MaxNLocator, FixedLocator, DictFormatter, FormatterPrettyPrint and GridFinder.get_grid_info.
import numpy as np
from matplotlib.transforms import Affine2D
from mpl_toolkits.axisartist import grid_finder as gf

for nbins, steps in [(5, None), (3, [1, 2, 5, 10]), (8, [1, 5, 10])]:
    loc = gf.MaxNLocator(nbins=nbins, steps=steps)
    levs, n, factor = loc(0.13, 7.9)
    print(nbins, np.round(levs, 4).tolist(), n, factor)
fixed = gf.FixedLocator([0, 0.5, 1, 1.5, 2.5])
print(fixed(0.2, 2))
fmt = gf.DictFormatter({0: "A", 1: "B", 2: "C"}, formatter=gf.FormatterPrettyPrint())
print(fmt("bottom", 1, [0, 1, 2, 3]))
pp = gf.FormatterPrettyPrint(useMathText=False)
print(pp("left", 1, [0.001, 0.5, 1000, 12345]))
pp2 = gf.FormatterPrettyPrint(useMathText=True)
print(pp2("left", 1, [1e-5, 2e-5]))
finder = gf.GridFinder(Affine2D().rotate_deg(30), grid_locator1=gf.MaxNLocator(4), grid_locator2=gf.MaxNLocator(4))
info = finder.get_grid_info(0, 0, 10, 10)
print(sorted(info.keys()))
print(np.round(info["extremes"].bounds, 3).tolist(), sorted(info["lon"]), sorted(info["lon"]["ticks"]))
print(len(info["lon"]["lines"]), len(info["lat"]["lines"]))
print([(float(t["level"]), round(float(t["loc"][0][0]), 3), round(float(t["loc"][1]), 1), t["label"]) for t in info["lon"]["ticks"]["bottom"]][:3])
ext = gf.ExtremeFinderSimple(20, 20)
print(np.round(ext(lambda x, y: (np.asarray(x) * 2, np.asarray(y) + 1), 0, 0, 1, 1), 4).tolist())
finder.update(grid_locator1=gf.FixedLocator([0, 5, 10]))
from matplotlib.transforms import Bbox
info2 = finder.get_grid_info(Bbox.from_extents(0, 0, 10, 10))
print(len(info2["lon"]["lines"]), [t["label"] for t in info2["lon"]["ticks"]["bottom"]])
print(finder.inv_transform_xy([1, 2], [3, 4]))
print(np.round(finder.transform_xy([1, 2], [3, 4]), 4).tolist())
