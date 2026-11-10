"""
The keyword arguments every artist takes that cross to pltrs (Rust) in a
form of their own: ``clip_path``, ``path_effects`` and ``picker``, as the
native property tables of lines, patches, collections and images take
them when an artist is made with them. A callable picker does not cross:
it stays with the Python artist (`keep_python_kwargs`), which the pick
calls, as `.Artist.set_picker` keeps it; so does the clip path's object.
`check_known` refuses the properties no setter takes, as matplotlib's
constructors do.
"""

from ._artist_clipping import clip_path_spec


def native_artist_kwargs(kwargs):
    """
    *kwargs* (aliases resolved) with ``clip_path``, ``path_effects`` and
    ``picker`` in native form; the others as they are.
    """
    native = dict(kwargs)
    if native.get("clip_path") is not None:
        native["clip_path"] = clip_path_spec(native["clip_path"])
    if "path_effects" in native:
        native["path_effects"] = [effect._native_spec()
                                  for effect in native["path_effects"]]
    if callable(native.get("picker")):
        del native["picker"]
    return native


def keep_python_kwargs(artist, kwargs):
    """
    Give *artist*, made natively with the keyword arguments *kwargs*, what
    of them stays with the Python artist; return *artist*.

    The callable picker they hold, which the pick calls, and any other
    picker as given (``picker=5`` reads back as ``5``, as matplotlib's
    ``set_picker`` keeps it); the clip path as `.Artist.set_clip_path`
    keeps it (its object, a patch followed as it changes). A `.Rectangle`
    sets only the clip box, which the Axes adding the artist replaces with
    its own patch's, as matplotlib's ``add_line`` does.
    """
    from .patches import Rectangle
    picker = kwargs.get("picker")
    if callable(picker):
        artist.set_picker(picker)
    elif "picker" in kwargs:
        artist._keep_picker(picker)
    clip_path = kwargs.get("clip_path")
    if isinstance(clip_path, Rectangle) and artist.axes is not None:
        artist.set_clip_path(artist.axes.patch)
    elif clip_path is not None:
        artist.set_clip_path(clip_path)
    return artist


def check_known(cls, kwargs):
    """Raise the `AttributeError` matplotlib's ``set`` of an artist of
    *cls* (the class matplotlib makes it of, a 2-D one for the artists it
    lifts into 3-D later) raises for the first property of *kwargs* none of
    its setters takes: matplotlib makes an artist with its properties before
    it adds it anywhere, where pltrs (Rust) makes it in its parent."""
    for name in kwargs:
        if not callable(getattr(cls, f"set_{name}", None)):
            raise AttributeError(f"{cls.__name__}.set() got an unexpected "
                                 f"keyword argument {name!r}", name=name)
