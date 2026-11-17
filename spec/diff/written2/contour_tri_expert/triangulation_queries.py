# Triangulation queries: edges, neighbors, masks, TrapezoidMapTriFinder lookups and calculate_plane_coefficients.
import numpy as np
import matplotlib.tri as mtri

x = np.array([0, 1, 2, 0, 1, 2, 0, 1, 2], float)
y = np.array([0, 0, 0, 1, 1, 1, 2, 2, 2], float) + 0.1 * np.array([0, 1, 0, 1, 0, 1, 0, 1, 0])
tri = mtri.Triangulation(x, y)
print(tri.triangles.shape, np.sort(tri.edges, axis=1).tolist()[:4], len(tri.edges))
print(tri.neighbors.tolist()[:3])
finder = tri.get_trifinder()
pts = [(0.2, 0.2), (1.5, 1.5), (5, 5), (1.0, 1.05)]
print([int(finder(px, py)) for px, py in pts])
z = x + 2 * y
print(np.round(tri.calculate_plane_coefficients(z), 4).tolist()[:2])
mask = np.zeros(len(tri.triangles), bool)
mask[[0, 3]] = True
tri.set_mask(mask)
print(tri.mask.sum(), len(tri.get_masked_triangles()), len(tri.edges), [int(tri.get_trifinder()(px, py)) for px, py in pts])
tri.set_mask(None)
explicit = mtri.Triangulation(x, y, triangles=[[0, 1, 3], [1, 4, 3], [1, 2, 4]])
print(explicit.triangles.tolist(), explicit.is_delaunay)
try:
    mtri.Triangulation([0, 1], [0, 1])
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    mtri.Triangulation(x, y, triangles=[[0, 1, 99]])
except ValueError as e:
    print("ValueError", str(e)[:60])
analyzer = mtri.TriAnalyzer(tri)
print(np.round(analyzer.scale_factors, 4).tolist(), np.round(analyzer.circle_ratios()[:4], 4).tolist())
print(analyzer.get_flat_tri_mask(min_circle_ratio=0.3).sum())
xt, yt = mtri.Triangulation(x, y).x, mtri.Triangulation(x, y).y
print(np.allclose(xt, x), np.allclose(yt, y))
