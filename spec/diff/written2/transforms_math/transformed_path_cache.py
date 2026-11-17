# TransformedPath caching and invalidation: change the transform after making it and check the re-evaluated vertices.
import numpy as np
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, TransformedPath, TransformedPatchPath
from matplotlib.patches import Rectangle

path = Path([(0, 0), (1, 0), (1, 1)], [Path.MOVETO, Path.LINETO, Path.LINETO])
aff = Affine2D().scale(2)
tp = TransformedPath(path, aff)
print(np.round(tp.get_fully_transformed_path().vertices, 6).tolist())
aff.translate(10, 0)
print(np.round(tp.get_fully_transformed_path().vertices, 6).tolist())
p2, affine = tp.get_transformed_path_and_affine()
print(np.round(p2.vertices, 6).tolist(), np.round(affine.get_matrix(), 6).tolist())
pts, aff2 = tp.get_transformed_points_and_affine()
print(type(pts).__name__, type(aff2).__name__)
print(np.round(tp.get_affine().get_matrix(), 6).tolist())
rect = Rectangle((1, 1), 2, 3, angle=30)
tpp = TransformedPatchPath(rect)
print(np.round(tpp.get_fully_transformed_path().vertices[:3], 6).tolist())
rect.set_width(4)
print(np.round(tpp.get_fully_transformed_path().vertices[:3], 6).tolist())
aff.clear().rotate_deg(90)
print(np.round(tp.get_fully_transformed_path().vertices, 6).tolist())
codes_only = TransformedPath(Path([(0, 0), (2, 2)]), Affine2D().skew_deg(10, 0))
print(np.round(codes_only.get_fully_transformed_path().vertices, 6).tolist(), codes_only.get_fully_transformed_path().codes)
rect.set_angle(0)
print(np.round(tpp.get_fully_transformed_path().vertices[:2], 6).tolist())
print(type(tpp).__name__, tpp.get_affine().is_affine)
print(np.round(TransformedPath(path, Affine2D()).get_fully_transformed_path().vertices, 6).tolist())
