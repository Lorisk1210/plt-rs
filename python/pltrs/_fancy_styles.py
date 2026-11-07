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

import functools
from numbers import Real
import types

from . import _unusable
from ._native import patches as _patches
from ._patch_styles import _native_spec


#: No mutation scale kept (see `FancyStylesMixin.set_mutation_scale`).
_NOT_KEPT = object()


def native_aspect(aspect):
    """The mutation *aspect* as pltrs (Rust) holds it: 1 for None, the
    float of a number, 1 for anything else, which the patch keeps instead
    (see `FancyStylesMixin._keep_mutation`)."""
    return float(aspect) if isinstance(aspect, Real) else 1.


def _style_object(style, registry, **kwargs):
    """*style* as an object of *registry*: a name (with *kwargs*) makes
    one, an object is kept as it is."""
    return registry(style, **kwargs) if isinstance(style, str) else style


class FancyStylesMixin:
    """What `.FancyBboxPatch` and `.FancyArrowPatch` share: their style
    objects and mutation (whose getters each class defines, as
    matplotlib's do, with docstrings of their own)."""

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)
        if FancyStylesMixin in cls.__bases__:
            # Each class defines the setters, as matplotlib's do: the
            # errors calling them name it.
            for name in ("set_mutation_scale", "set_mutation_aspect"):
                if name not in vars(cls):
                    setattr(cls, name, _owned(vars(FancyStylesMixin)[name], cls))

    def _set_style(self, key, style, registry, native_set, native_get):
        """Give the patch *style* of *registry*, set by *native_set* and
        described back by *native_get*; return the style object."""
        native_set(self._native, _native_spec(style, registry, self))
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
        """
        Keep the mutation *scale* and *aspect* as given (an integer stays
        one), 1 for an aspect of None. An aspect that is no number, which
        matplotlib keeps as it is, the patch keeps instead of pltrs (Rust)
        and fails on where matplotlib's computes with it
        (``_use_mutation_aspect``, see `._unusable`).
        """
        self._given_mutation = (scale, 1 if aspect is None else aspect)
        if aspect is None or isinstance(aspect, Real):
            _unusable.drop(self, "mutation_aspect")
        else:
            _unusable.keep(self, "mutation_aspect", aspect, self._use_mutation_aspect)

    def _mutation(self, index):
        """The mutation scale (*index* 0) or aspect (1): as given while
        pltrs (Rust) has it, else pltrs (Rust)'s; a scale the patch keeps
        (see `set_mutation_scale`) as it is."""
        if index == 0:
            kept = _unusable.kept(self, "mutation_scale", _NOT_KEPT)
            if kept is not _NOT_KEPT:
                return kept
        native = _patches.get_mutation(self._native)[index]
        given = self.__dict__.get("_given_mutation", (None, None))[index]
        return given if isinstance(given, Real) and float(given) == native else native

    def set_mutation_scale(self, scale):
        """
        Set the mutation scale.

        Parameters
        ----------
        scale : float
        """
        if isinstance(scale, Real):
            _patches.set_mutation(self._native, float(scale), None)
            _unusable.drop(self, "mutation_scale")
        else:
            # matplotlib keeps it, and fails to mutate the patch with it
            # (``_use_mutation_scale``, see `._unusable`).
            _unusable.keep(self, "mutation_scale", scale, self._use_mutation_scale)
        self._keep_mutation(scale, self.get_mutation_aspect())
        self.stale = True

    def set_mutation_aspect(self, aspect):
        """
        Set the aspect ratio of the bbox mutation.

        Parameters
        ----------
        aspect : float
        """
        _patches.set_mutation(self._native, None, native_aspect(aspect))
        self._keep_mutation(self._mutation(0), aspect)
        self.stale = True

    def _kept_mutation_scale(self):
        """The mutation scale: one the patch keeps (see
        `set_mutation_scale`), else `_mutation`'s."""
        return _unusable.kept(self, "mutation_scale", self._mutation(0))

    def _kept_mutation_aspect(self):
        """The mutation aspect: one the patch keeps (see `_keep_mutation`),
        else `_mutation`'s."""
        aspect = _unusable.kept(self, "mutation_aspect", None)
        return self._mutation(1) if aspect is None else aspect


def _owned(function, cls):
    """A copy of *function*, a method of `FancyStylesMixin`, as a method of
    *cls*."""
    copy = types.FunctionType(function.__code__, function.__globals__, function.__name__,
                              function.__defaults__, function.__closure__)
    functools.update_wrapper(copy, function)
    del copy.__wrapped__
    copy.__qualname__ = f"{cls.__qualname__}.{function.__name__}"
    copy.__module__ = cls.__module__
    return copy
