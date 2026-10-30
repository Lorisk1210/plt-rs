"""
Builtin colormaps, colormap handling utilities, and the `ScalarMappable` mixin.

.. seealso::

  :doc:`/gallery/color/colormap_reference` for a list of builtin colormaps.

  :ref:`colormap-manipulation` for examples of how to make
  colormaps.

  :ref:`colormaps` an in-depth discussion of choosing
  colormaps.

  :ref:`colormapnorms` for more details about data normalization.

pltrs keeps matplotlib's module (from matplotlib 3.11.2, under the
Matplotlib License); the definitions of the built-in colormaps come from
pltrs (Rust) (``pltrs::colors::colormaps`` and ``pltrs::colors::multivar``),
which holds the data of matplotlib's ``_cm``, ``_cm_listed``,
``_cm_multivar`` and ``_cm_bivar``.
"""

from collections.abc import Mapping

import numpy as np

from . import _api, colors
from ._native import cm as _cm
from .colorizer import _ScalarMappable as ScalarMappable  # noqa: F401


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams[key]


_LUTSIZE = _rc('image.lut')


class _BuiltinSegment:
    """
    A channel of a built-in `.LinearSegmentedColormap` defined by a
    function (gnuplot's formulae, cubehelix, ...), which pltrs (Rust)
    evaluates: called with an array of values in [0, 1], it returns the
    channel's values there.
    """

    def __init__(self, cmap_name, channel):
        self._cmap_name = cmap_name
        self._channel = channel

    def __call__(self, x):
        return _cm.builtin_segment(self._cmap_name, self._channel,
                                   np.asarray(x, dtype=float))

    def __repr__(self):
        return f"<{self._channel} channel of colormap {self._cmap_name!r}>"


def _from_spec(spec):
    """The `.Colormap` a native colormap spec describes."""
    name = spec['name']
    if 'colors' in spec:
        return colors.ListedColormap(spec['colors'].tolist(), name)
    segmentdata = {
        channel: (_BuiltinSegment(name, channel) if rows is None else rows)
        for channel, rows in spec['segmentdata'].items()}
    return colors.LinearSegmentedColormap(name, segmentdata, spec['N'])


def _gen_cmap_registry():
    """
    Generate a dict mapping standard colormap names to standard colormaps, as
    well as the reversed colormaps.
    """
    cmap_d = {name: _from_spec(_cm.builtin_colormap(name))
              for name in _cm.colormap_names()}
    # Generate reversed cmaps.
    for cmap in list(cmap_d.values()):
        rmap = cmap.reversed()
        cmap_d[rmap.name] = rmap
    return cmap_d


def _gen_multivar_cmaps():
    """The multivariate colormaps (matplotlib's ``_cm_multivar``)."""
    families = {}
    for name in _cm.multivar_names():
        mode, specs = _cm.multivar(name)
        families[name] = colors.MultivarColormap(
            [_from_spec(spec) for spec in specs], mode, name=name)
    return families


def _gen_bivar_cmaps():
    """The bivariate colormaps (matplotlib's ``_cm_bivar``)."""
    cmaps = {}
    for name in _cm.bivar_names():
        patch, shape, origin = _cm.bivar(name)
        cmaps[name] = colors.SegmentedBivarColormap(
            patch, 256, shape, origin, name=name)
    return cmaps


class ColormapRegistry(Mapping):
    r"""
    Container for colormaps that are known to Matplotlib by name.

    The universal registry instance is `matplotlib.colormaps`. There should be
    no need for users to instantiate `.ColormapRegistry` themselves.

    Read access uses a dict-like interface mapping names to `.Colormap`\s::

        import matplotlib as mpl
        cmap = mpl.colormaps['viridis']

    Returned `.Colormap`\s are copies, so that their modification does not
    change the global definition of the colormap.

    Additional colormaps can be added via `.ColormapRegistry.register`::

        mpl.colormaps.register(my_colormap)

    To get a list of all registered colormaps, you can do::

        from matplotlib import colormaps
        list(colormaps)
    """
    def __init__(self, cmaps):
        self._cmaps = cmaps
        self._builtin_cmaps = tuple(cmaps)

    def __contains__(self, item):
        return item in self._cmaps

    def __getitem__(self, item):
        cmap = _api.getitem_checked(self._cmaps, colormap=item, _error_cls=KeyError)
        return cmap.copy()

    def __iter__(self):
        return iter(self._cmaps)

    def __len__(self):
        return len(self._cmaps)

    def __str__(self):
        return ('ColormapRegistry; available colormaps:\n' +
                ', '.join(f"'{name}'" for name in self))

    def __call__(self):
        """
        Return a list of the registered colormap names.

        This exists only for backward-compatibility in `.pyplot` which had a
        ``plt.colormaps()`` method. The recommended way to get this list is
        now ``list(colormaps)``.
        """
        return list(self)

    def register(self, cmap, *, name=None, force=False):
        """
        Register a new colormap.

        The colormap name can then be used as a string argument to any ``cmap``
        parameter in Matplotlib. It is also available in ``pyplot.get_cmap``.

        The colormap registry stores a copy of the given colormap, so that
        future changes to the original colormap instance do not affect the
        registered colormap. Think of this as the registry taking a snapshot
        of the colormap at registration.

        Parameters
        ----------
        cmap : matplotlib.colors.Colormap
            The colormap to register.

        name : str, optional
            The name for the colormap. If not given, ``cmap.name`` is used.

        force : bool, default: False
            If False, a ValueError is raised if trying to overwrite an already
            registered name. True supports overwriting registered colormaps
            other than the builtin colormaps.
        """
        _api.check_isinstance(colors.Colormap, cmap=cmap)

        name = name or cmap.name
        if name in self:
            if not force:
                # don't allow registering an already existing cmap
                # unless explicitly asked to
                raise ValueError(
                    f'A colormap named "{name}" is already registered.')
            elif name in self._builtin_cmaps:
                # We don't allow overriding a builtin.
                raise ValueError("Re-registering the builtin cmap "
                                 f"{name!r} is not allowed.")

            # Warn that we are updating an already existing colormap
            _api.warn_external(f"Overwriting the cmap {name!r} "
                               "that was already in the registry.")

        self._cmaps[name] = cmap.copy()
        # Someone may set the extremes of a builtin colormap and want to register it
        # with a different name for future lookups. The object would still have the
        # builtin name, so we should update it to the registered name
        if self._cmaps[name].name != name:
            self._cmaps[name].name = name

    def unregister(self, name):
        """
        Remove a colormap from the registry.

        You cannot remove built-in colormaps.

        If the named colormap is not registered, returns with no error, raises
        if you try to de-register a default colormap.

        .. warning::

            Colormap names are currently a shared namespace that may be used
            by multiple packages. Use `unregister` only if you know you
            have registered that name before. In particular, do not
            unregister just in case to clean the name before registering a
            new colormap.

        Parameters
        ----------
        name : str
            The name of the colormap to be removed.

        Raises
        ------
        ValueError
            If you try to remove a default built-in colormap.
        """
        if name in self._builtin_cmaps:
            raise ValueError(f"cannot unregister {name!r} which is a builtin "
                             "colormap.")
        self._cmaps.pop(name, None)

    def get_cmap(self, cmap):
        """
        Return a color map specified through *cmap*.

        Parameters
        ----------
        cmap : str or `~matplotlib.colors.Colormap` or None

            - if a `.Colormap`, return it
            - if a string, look it up in ``mpl.colormaps``
            - if None, return the Colormap defined in :rc:`image.cmap`

        Returns
        -------
        Colormap
        """
        # get the default color map
        if cmap is None:
            return self[_rc("image.cmap")]

        # if the user passed in a Colormap, simply return it
        if isinstance(cmap, colors.Colormap):
            return cmap
        if isinstance(cmap, str):
            _api.check_in_list(sorted(_colormaps), cmap=cmap)
            # otherwise, it must be a string so look it up
            return self[cmap]
        raise TypeError(
            'get_cmap expects None or an instance of a str or Colormap . ' +
            f'you passed {cmap!r} of type {type(cmap)}'
        )


class _NativeColormapRegistry(ColormapRegistry):
    """
    The registry of `matplotlib.colormaps`, whose registrations pltrs
    (Rust) mirrors, so that :rc:`image.cmap` may name them.
    """

    def register(self, cmap, *, name=None, force=False):
        super().register(cmap, name=name, force=force)
        name = name or cmap.name
        _cm.register(name, self._cmaps[name])

    def unregister(self, name):
        super().unregister(name)
        _cm.unregister(name)


# public access to the colormaps should be via `matplotlib.colormaps`. For now,
# we still create the registry here, but that should stay an implementation
# detail.
_colormaps = _NativeColormapRegistry(_gen_cmap_registry())
globals().update(_colormaps)

_multivar_colormaps = ColormapRegistry(_gen_multivar_cmaps())


def _from_native(native):
    """
    The colormap of a mapping pltrs (Rust) made, given as ``(spec, under,
    over, bad)`` (``_native.cm``): the registered colormap of its name, or
    the one its spec describes, with its extreme colours; None for none.
    """
    if native is None:
        return None
    spec, *extremes = native
    name = spec['name']
    cmap = _colormaps[name] if name in _colormaps else _from_spec(spec)
    own = (cmap.get_under(), cmap.get_over(), cmap.get_bad())
    if all(np.allclose(a, b) for a, b in zip(own, extremes)):
        return cmap
    under, over, bad = extremes
    return cmap.with_extremes(under=under, over=over, bad=bad)

_bivar_colormaps = ColormapRegistry(_gen_bivar_cmaps())
