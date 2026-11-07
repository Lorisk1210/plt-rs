"""
What `.Artist` gives its subclasses as they are defined, from matplotlib
3.11.2's ``artist`` module under the Matplotlib License: a ``set`` method
whose signature and docstring list the subclass' properties, and a
``draw`` method that stops the rasterization of artists drawn after a
rasterized one.

pltrs (Rust) draws the artists it holds itself; ``draw`` is what a Python
subclass overrides to draw through a renderer.

matplotlib writes the signature and docstring of every subclass' ``set``
as the class is defined, which costs most of the time ``import
matplotlib.pyplot`` spends in matplotlib's own code. pltrs writes them the
first time the class' ``set`` is looked up (`_LazySet`), which gives the
same function, signature and docstring.
"""

from functools import wraps
from inspect import Parameter, Signature

from . import _api


class ClassZOrder:
    """
    ``zorder``: on a class, the z-order its artists start with
    (``Line2D.zorder``, matplotlib's class attribute); on an artist, the
    artist's (``line.zorder``, read and set through `get_zorder` and
    `set_zorder`). A class setting ``zorder = n`` gets one of these.
    """

    def __init__(self, default):
        self.default = default

    def __get__(self, artist, owner=None):
        return self.default if artist is None else artist.get_zorder()

    def __set__(self, artist, level):
        artist.set_zorder(level)


def _prevent_rasterization(draw):
    """
    Wrap the *draw* method of an artist which cannot be rasterized: drawn
    outside any rasterized parent, it stops the rasterization an artist
    drawn before it started.
    """

    @wraps(draw)
    def draw_wrapper(artist, renderer, *args, **kwargs):
        if renderer._raster_depth == 0 and renderer._rasterizing:
            # Only stop when we are not in a rasterized parent
            # and something has been rasterized since last stop.
            renderer.stop_rasterizing()
            renderer._rasterizing = False

        return draw(artist, renderer, *args, **kwargs)

    draw_wrapper._supports_rasterization = False
    return draw_wrapper


def _is_alias(method):
    """Whether *method* is an alias of another (`.ArtistInspector.is_alias`,
    as a class defining it documents it)."""
    doc = method.__doc__ or ""
    # The start of a long docstring, stripped, unless spaces fill it.
    head = doc[:80].lstrip()
    if len(head) < len("Alias for ") and len(doc) > 80:
        head = doc.lstrip()
    return head.startswith("Alias for ")


class _Definition:
    """
    What an artist class held when it was defined, which matplotlib's
    ``__init_subclass__`` reads to write the docstring of the class' ``set``
    then: the names of its own accessors (``set_*``, ``get_*``), and those
    of its own setters without a docstring, whose docstring Python cannot
    look up on a base class yet (the module does not name the class before
    its definition ends).
    """

    def __init__(self, cls):
        names, undocumented, aliases, setters = [], [], [], {}
        for name, value in vars(cls).items():
            if not name.startswith(("set_", "get_")):
                continue
            names.append(name)
            if not callable(value):
                continue
            alias = _is_alias(value)
            if alias:
                aliases.append(name)
            if name.startswith("set_"):
                if value.__doc__ is None:
                    undocumented.append(name)
                if not alias:
                    setters[name] = value
        self.names = frozenset(names)
        self.undocumented = frozenset(undocumented)
        #: The alias accessors the class defined.
        self.aliases = frozenset(aliases)
        #: The setters the class defined that were no aliases then, which
        #: its decorators may make aliases of (``set_xlim3d = set_xlim``).
        self.setters = setters

    def inspector(self, cls):
        """An `.ArtistInspector` of *cls* as it was defined."""
        from ._artist_inspector import ArtistInspector, _setter_valid_values
        definition = self

        class DefinitionInspector(ArtistInspector):
            def get_aliases(self):
                # Not yet the aliases the class' own decorators add: only
                # the alias methods the class defined, or inherited.
                def was_alias(name):
                    if name in definition.names:
                        return name in definition.aliases
                    return any(name in vars(klass) and self.is_alias(vars(klass)[name])
                               for klass in cls.__mro__[1:])
                return {
                    prop: kept for prop, aliases in super().get_aliases().items()
                    if (kept := {alias for alias in aliases
                                 if any(was_alias(f"{prefix}{alias}")
                                        for prefix in ("set_", "get_"))})}

            def get_setters(self):
                # And the setters the class defined that its decorators
                # made aliases of afterwards.
                setters = set(super().get_setters())
                setters.update(
                    name[4:] for name in definition.setters
                    if self.number_of_parameters(getattr(self.o, name)) >= 2)
                return sorted(setters)

            def get_valid_values(self, attr):
                func = getattr(self.o, f"set_{attr}", None)
                defined = definition.setters.get(f"set_{attr}")
                if defined is not None and func is not defined and self.is_alias(func):
                    # A setter its decorators made an alias of: as it was.
                    return _setter_valid_values(defined)
                if (not hasattr(func, "_kwarg_doc")
                        and f"set_{attr}" in definition.undocumented):
                    return 'unknown'
                return super().get_valid_values(attr)

        return DefinitionInspector(cls)


class _LazySet:
    """
    The ``set`` of an artist class until it is first looked up: then it
    makes the class' ``set`` as matplotlib's ``__init_subclass__`` makes it,
    puts it in the class in its own place, and returns it (bound to the
    instance it was looked up on). The function is named as matplotlib's
    ``artist`` module names it while the class is being defined: by the
    class' ``__qualname__`` then (a class factory renames its classes
    afterwards). `.Artist`'s own ``set`` (*function*) gets its signature
    and docstring so, when first looked up, as matplotlib gives them when
    its module is imported.
    """

    def __init__(self, cls, function=None):
        self._cls = cls
        self._qualname = f"{cls.__qualname__}.set"
        self._function = function

    def __get__(self, instance, owner=None):
        function = self._make()
        return function if instance is None else function.__get__(instance, owner)

    def _make(self):
        cls = self._cls
        if not isinstance(cls.__dict__.get("set"), _LazySet):
            return cls.__dict__["set"]  # Made by an earlier lookup.
        function = self._function
        if function is None:
            from .artist import Artist
            function = lambda self, **kwargs: Artist.set(self, **kwargs)  # noqa: E731
            function.__name__ = "set"
            function.__qualname__ = self._qualname
            function.__module__ = Artist.__module__
        type.__setattr__(cls, "set", function)
        cls._update_set_signature_and_docstring()
        return function


def _inherits_generated_set(cls):
    """Whether the ``set`` *cls* would inherit is a generated one (made,
    or still to make), rather than one a class defined itself."""
    for klass in cls.__mro__:
        if "set" in klass.__dict__:
            inherited = klass.__dict__["set"]
            return (isinstance(inherited, _LazySet)
                    or hasattr(inherited, "_autogenerated_signature"))
    return False


class SubclassingMixin:
    """The class-level machinery of `.Artist`."""

    #: Whether drawing artists of the class ignores their rasterization:
    #: true for the classes of this package whose matplotlib ``draw`` is
    #: not decorated with `.allow_rasterization`.
    _ignores_rasterization = False

    def _rasterization_ignored(self):
        """Whether drawing the artist ignores its rasterization (so that
        asking for it warns, as matplotlib's ``set_rasterized`` does): its
        class' own setting, or for a ``draw`` of the program's, whether
        that is not decorated with `.allow_rasterization`."""
        from ._program_classes import takes_from_program
        cls = type(self)
        if takes_from_program(cls, "draw"):
            return not getattr(cls.draw, "_supports_rasterization", False)
        return cls._ignores_rasterization

    _PROPERTIES_EXCLUDED_FROM_SET = [
        'navigate_mode',  # not a user-facing function
        'figure',         # changing the figure is such a profound operation
                          # that we don't want this in set()
        '3d_properties',  # cannot be used as a keyword due to leading digit
    ]

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)
        zorder = vars(cls).get("zorder")
        if isinstance(zorder, (int, float)) and not isinstance(zorder, bool):
            cls.zorder = ClassZOrder(zorder)
        if SubclassingMixin in cls.__bases__:
            return  # `.Artist` itself, as matplotlib's runs for subclasses.
        # Decorate draw() method so that all artists are able to stop
        # rastrization when necessary. If the artist's draw method is already
        # decorated (has a `_supports_rasterization` attribute), it won't be
        # decorated.
        if not hasattr(cls.draw, "_supports_rasterization"):
            cls.draw = _prevent_rasterization(cls.draw)

        # Inject custom set() methods into the subclass with signature and
        # docstring based on the subclasses' properties, made when first
        # looked up.
        if not _inherits_generated_set(cls):
            # Don't overwrite cls.set if the subclass or one of its parents
            # has defined a set method set itself.
            return
        cls.set = _LazySet(cls)
        cls._definition = _Definition(cls)

    @classmethod
    def _update_set_signature_and_docstring(cls):
        """
        Update the signature of the set function to list all properties
        as keyword arguments.

        Property aliases are not listed in the signature for brevity, but
        are still accepted as keyword arguments.
        """
        from ._artist_inspector import ArtistInspector, _properties_doc
        definition = vars(cls).get("_definition")
        inspector = (definition.inspector(cls) if definition is not None
                     else ArtistInspector(cls))
        cls.set.__signature__ = Signature(
            [Parameter("self", Parameter.POSITIONAL_OR_KEYWORD),
             *[Parameter(prop, Parameter.KEYWORD_ONLY, default=_api.UNSET)
               for prop in inspector.get_setters()
               if prop not in cls._PROPERTIES_EXCLUDED_FROM_SET]])
        cls.set._autogenerated_signature = True

        cls.set.__doc__ = ("""\
Set multiple properties at once.

::

    a.set(a=A, b=B, c=C)

is equivalent to ::

    a.set_a(A)
    a.set_b(B)
    a.set_c(C)

In addition to the full property names, aliases are also supported, e.g.
``set(lw=2)`` is equivalent to ``set(linewidth=2)``, but it is an error
to pass both simultaneously.

The order of the individual setter calls matches the order of parameters
in ``set()``. However, most properties do not depend on each other so
that order is rarely relevant.

Supported properties are

""" + _properties_doc(inspector))
