# The type stub of matplotlib._path: matplotlib 3.11.2's, under the
# Matplotlib License, as matplotlib installs it
# (python/tools/matplotlib_stubs.py).

from collections.abc import Sequence

import numpy as np

from .transforms import BboxBase

def affine_transform(points: np.ndarray, trans: np.ndarray) -> np.ndarray: ...
def count_bboxes_overlapping_bbox(bbox: BboxBase, bboxes: Sequence[BboxBase]) -> int: ...
