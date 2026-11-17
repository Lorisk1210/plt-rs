# colors module tables older code uses: cnames, CSS4/XKCD/TABLEAU/BASE dicts, ColorConverter and to_rgba/to_hex conversions.
import numpy as np
import matplotlib
from matplotlib import colors

print(len(colors.cnames), colors.cnames["aliceblue"], colors.cnames is colors.CSS4_COLORS)
print(len(colors.XKCD_COLORS), colors.XKCD_COLORS["xkcd:sky blue"], len(colors.TABLEAU_COLORS), sorted(colors.BASE_COLORS))
cc = colors.ColorConverter()
print(cc.to_rgb("r"), cc.to_rgba("b", 0.5), colors.colorConverter.to_rgb("g"))
for spec in ["C0", "tab:orange", "xkcd:red", "0.25", "#abc", "#abcd", "#aabbccdd", (0.1, 0.2, 0.3), ("red", 0.3), "none"]:
    print(repr(spec), np.round(colors.to_rgba(spec), 4).tolist(), colors.to_hex(spec, keep_alpha=True))
print(colors.same_color("red", "#ff0000"), colors.same_color(["r", "g"], ["red", "green"]))
print(colors.is_color_like("purple"), colors.is_color_like("purplish"), colors.is_color_like("C10"))
print(np.round(colors.to_rgba_array(["r", "g", (0, 0, 1, 0.5)]), 3).tolist())
print(np.round(colors.rgb_to_hsv([[1, 0, 0], [0, 1, 1]]), 4).tolist(), np.round(colors.hsv_to_rgb([0.5, 1, 1]), 4).tolist())
try:
    colors.to_rgba("notacolor")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(colors.to_hex((0.5, 0.5, 0.5)), colors.to_rgb("C9"))
print(matplotlib.colors.get_named_colors_mapping()["red"])
cmap = matplotlib.colormaps["viridis"]
print(np.round(cmap(0.5), 4).tolist(), cmap.N, colors.to_hex(cmap(0.0)))
lc = colors.LinearSegmentedColormap.from_list("rg", ["red", "green"], N=5)
print([colors.to_hex(lc(i)) for i in range(5)])
lc2 = colors.ListedColormap(["#000000", "#ffffff"])
print(lc2.N, lc2.colors, colors.to_hex(lc2(1.0)))
print(np.round(colors.Normalize(2, 4)([1, 3, 5]), 3).tolist(), colors.Normalize(2, 4).inverse(0.5))
print(np.round(colors.LogNorm(1, 100)([1, 10, 100]), 3).tolist())
print(np.round(colors.LightSource(315, 45).hillshade(np.eye(3)), 3).tolist()[0])
print(len(colors.ColorConverter.colors) > 1000)
