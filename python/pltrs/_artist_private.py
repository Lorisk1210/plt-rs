"""
The private attributes of matplotlib's `.Artist` that released libraries
read, which pltrs keeps elsewhere: ``_axes``, the attribute behind
matplotlib's `.Artist.axes` (nilearn's tests read ``image._axes``).
"""


class PrivateAttributesMixin:
    """matplotlib's private attributes of an artist, where nothing set
    them."""

    def __getattr__(self, name):
        if name == "_axes":
            from .figure import FigureBase
            # A figure has none: matplotlib's deletes it, its ``axes`` being
            # the list of its Axes.
            if not isinstance(self, FigureBase):
                # The Axes the artist is in, as its ``axes`` gives it (an
                # Axes itself, as matplotlib's ``self.axes = self`` sets it).
                return type(self).axes.fget(self)
        prop = getattr(type(self), name, None)
        if isinstance(prop, property) and prop.fget is not None:
            # A property whose getter raised: its error is the one, as
            # without this hook (a figure pyplot never numbered).
            return prop.fget(self)
        raise AttributeError(
            f"'{type(self).__name__}' object has no attribute '{name}'",
            name=name, obj=self)
