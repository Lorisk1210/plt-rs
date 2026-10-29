"""
Groups of artists made by one plotting call (matplotlib's ``container``).

Containers are plain Python tuples of artist objects.
"""


class Container(tuple):
    """
    Base class for containers.

    Containers are classes that collect semantically related Artists such as
    the bars of a bar plot.
    """

    def __repr__(self):
        return f"<{type(self).__name__} object of {len(self)} artists>"

    def __new__(cls, *args, **kwargs):
        return tuple.__new__(cls, args[0])

    def __init__(self, kl, label=None):
        self._label = label

    def get_label(self):
        """Return the label used for this container in the legend."""
        return self._label

    def get_children(self):
        """Return the artists of the container."""
        return [child for child in self if child is not None]


class BarContainer(Container):
    """
    Container for the artists of bar plots (e.g. created by `.Axes.bar`).

    Attributes
    ----------
    patches : list of :class:`~matplotlib.patches.Rectangle`
        The artists of the bars.
    datavalues : None or array-like
        The underlying data values corresponding to the bars.
    orientation : {'vertical', 'horizontal'}, default: None
        If 'vertical', the bars are assumed to be vertical.
    """

    def __init__(self, patches, errorbar=None, *, datavalues=None,
                 orientation=None, **kwargs):
        self.patches = patches
        self.errorbar = errorbar
        self.datavalues = datavalues
        self.orientation = orientation
        super().__init__(patches, **kwargs)
