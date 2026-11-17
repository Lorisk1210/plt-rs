# Combine readback extents with Bbox operations: union, intersection, overlaps, expanded, padded, anchored and count_overlaps.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
texts = [ax.text(0.1 + 0.15 * k, 0.5 + 0.05 * (k % 2), f"label {k}") for k in range(5)]
fig.canvas.draw()
r = fig.canvas.get_renderer()
boxes = [t.get_window_extent(r) for t in texts]
u = Bbox.union(boxes)
print([round(v, 1) for v in u.bounds])
pairs = [(i, j) for i in range(5) for j in range(i + 1, 5) if boxes[i].overlaps(boxes[j])]
print(pairs)
inter = Bbox.intersection(boxes[0], boxes[1])
print(None if inter is None else [round(v, 1) for v in inter.bounds])
print(boxes[2].count_overlaps(boxes), boxes[2].count_contains([(b.x0 + 1, b.y0 + 1) for b in boxes]))
print([round(v, 1) for v in boxes[0].expanded(1.5, 2).bounds], [round(v, 1) for v in boxes[0].padded(3).bounds])
print([round(v, 1) for v in boxes[0].anchored("C", ax.bbox).bounds], [round(v, 1) for v in boxes[0].shrunk(0.5, 0.5).bounds])
print([round(v, 1) for v in boxes[0].translated(10, -10).bounds], [round(v, 1) for v in boxes[0].transformed(ax.transAxes.inverted()).bounds])
print(boxes[0].fully_overlaps(boxes[1]), boxes[0].contains(boxes[0].x0 + 1, boxes[0].y0 + 1), boxes[0].containsx(0))
frozen = boxes[0].frozen()
print(type(frozen).__name__, frozen == boxes[0], [round(v, 1) for v in frozen.splitx(0.5)[0].bounds])
print([round(v, 1) for v in Bbox.from_extents(*boxes[1].extents).bounds], boxes[1].minpos.round(1).tolist())
print(round(boxes[0].width, 1), round(boxes[0].height, 1), boxes[0].size.round(1).tolist(), boxes[0].p0.round(1).tolist())
big = Bbox.null()
for b in boxes:
    big.update_from_data_xy(b.get_points(), ignore=False)
print([round(v, 1) for v in big.bounds], Bbox.null().bounds)
print([round(v, 3) for v in Bbox.unit().bounds], Bbox([[0, 0], [1, 1]]).corners().tolist()[:2])
