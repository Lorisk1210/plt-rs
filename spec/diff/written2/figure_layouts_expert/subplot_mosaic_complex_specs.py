# subplot_mosaic with nested lists, empty sentinels, per-subplot kwargs, width/height ratios and string specs; labels and positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

mosaic = [["main", "main", "side"], ["main", "main", "side2"], [".", "bottom", "bottom"]]
fig, d = plt.subplot_mosaic(mosaic, figsize=(6, 5), width_ratios=[1, 2, 1], height_ratios=[2, 1, 1],
                            per_subplot_kw={"side": {"projection": "polar"}, ("side2", "bottom"): {"facecolor": "0.9"}},
                            gridspec_kw={"wspace": 0.3}, empty_sentinel=".")
fig.canvas.draw()
for k in sorted(d):
    print(k, type(d[k]).__name__, [round(v, 3) for v in d[k].get_position().bounds], d[k].get_label())
nested = [["a", [["b1", "b2"], ["b3", "b4"]]], ["c", "c"]]
fig2, d2 = plt.subplot_mosaic(nested, figsize=(5, 4))
print(sorted(d2), [round(v, 3) for v in d2["b4"].get_position().bounds])
fig3, d3 = plt.subplot_mosaic("AAB;CDB", sharex=True)
print(sorted(d3), d3["A"].get_shared_x_axes().joined(d3["A"], d3["D"]))
fig4, d4 = plt.subplot_mosaic("""
    xx.
    yzz
""")
print(sorted(d4))
try:
    plt.subplot_mosaic([["a", "b"], ["b", "a"]])
except ValueError as e:
    print("ValueError", str(e)[:60])
sfig = plt.figure()
subs = sfig.subfigures(1, 2)
sd = subs[1].subplot_mosaic("AB")
print(sorted(sd), sd["A"].get_figure() is subs[1], sd["A"].get_figure(root=True) is sfig)
plt.close("all")
