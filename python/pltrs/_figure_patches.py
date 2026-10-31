"""
The patches drawn on a figure by being appended to its list (matplotlib's
``Figure.patches``).

matplotlib's figure draws the patches its ``patches`` list holds as they
are: a patch appended without a transform of its own is drawn in display
pixels (its transform is the identity) and unclipped. pltrs (Rust) keeps
the artists of a figure; the list `FigureBase.patches` gives adds a patch
appended to it to the figure in that way.
"""

from .transforms import IdentityTransform


class FigurePatches(list):
    """The patches appended to *figure*, in a list drawing those appended
    to it on the figure."""

    def __init__(self, figure):
        super().__init__()
        self._figure = figure

    def append(self, patch):
        """Draw *patch* on the figure, in display pixels unless it has a
        transform of its own, and add it to the list."""
        if not patch.is_transform_set():
            patch.set_transform(IdentityTransform())
        patch._add_to_figure(self._figure, False)
        super().append(patch)
