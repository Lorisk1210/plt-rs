"""
The styles and mutation of fancy patches (matplotlib's
``FancyBboxPatch.set_boxstyle``, ``FancyArrowPatch.set_arrowstyle``,
``set_connectionstyle``, ``set_mutation_scale``, ``set_mutation_aspect``
and their getters).

pltrs (Rust) keeps a patch's style as its description
(``_native.patches.get_boxstyle``, ...): the style object a setter was
given comes back while the patch still has its style, and one made from
the description pltrs (Rust) gives otherwise (the patches pltrs (Rust)
makes itself, those their owner restyled).
"""

from ._native import patches as _patches
from ._patch_styles import _native_spec


def _style_object(style, registry, **kwargs):
    """*style* as an object of *registry*: a name (with *kwargs*) makes
    one, an object is kept as it is."""
    return registry(style, **kwargs) if isinstance(style, str) else style


class FancyStylesMixin:
    """What `.FancyBboxPatch` and `.FancyArrowPatch` share: their style
    objects and mutation."""

    def _set_style(self, key, style, registry, native_set, native_get):
        """Give the patch *style* of *registry*, set by *native_set* and
        described back by *native_get*; return the style object."""
        native_set(self._native, _native_spec(style, registry))
        self.__dict__[key] = (style, native_get(self._native))
        self.stale = True
        return style

    def _get_style(self, key, registry, native_get):
        """The style object of *registry* the patch has (see the module)."""
        description = native_get(self._native)
        style, kept = self.__dict__.get(key, (None, None))
        if kept != description:
            style = registry(description)
            self.__dict__[key] = (style, description)
        return style

    def _keep_mutation(self, scale, aspect):
        """Keep the mutation *scale* and *aspect* as given (an integer
        stays one), 1 for an aspect of None."""
        self._given_mutation = (scale, 1 if aspect is None else aspect)

    def _mutation(self, index):
        """The mutation scale (*index* 0) or aspect (1): as given while
        pltrs (Rust) has it, else pltrs (Rust)'s."""
        native = _patches.get_mutation(self._native)[index]
        given = self.__dict__.get("_given_mutation", (None, None))[index]
        return given if given is not None and float(given) == native else native

    def set_mutation_scale(self, scale):
        """
        Set the mutation scale.

        Parameters
        ----------
        scale : float
        """
        _patches.set_mutation(self._native, float(scale), None)
        self._keep_mutation(scale, self._mutation(1))
        self.stale = True

    def get_mutation_scale(self):
        """Return the mutation scale."""
        return self._mutation(0)

    def set_mutation_aspect(self, aspect):
        """
        Set the aspect ratio of the mutation.

        Parameters
        ----------
        aspect : float
        """
        _patches.set_mutation(self._native, None,
                              1. if aspect is None else float(aspect))
        self._keep_mutation(self._mutation(0), aspect)
        self.stale = True

    def get_mutation_aspect(self):
        """Return the aspect ratio of the mutation."""
        return self._mutation(1)
