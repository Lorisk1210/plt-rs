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

from ._artist_clipping import clip_path_spec, clipped_as_added


def native_artist_kwargs(kwargs):
    """
    *kwargs* (aliases resolved) with ``clip_path``, ``path_effects`` and
    ``picker`` in native form, without ``clip_box``; the others as they
    are.
    """
    native = dict(kwargs)
    if native.get("clip_path") is not None:
        native["clip_path"] = clip_path_spec(native["clip_path"])
    if "path_effects" in native:
        native["path_effects"] = [effect._native_spec()
                                  for effect in native["path_effects"]]
    if callable(native.get("picker")):
        del native["picker"]
    # The clip box stays with the Python artist (`keep_python_kwargs`).
    native.pop("clip_box", None)
    return native


def keep_python_kwargs(artist, kwargs):
    """
    Give *artist*, made natively with the keyword arguments *kwargs*, what
    of them stays with the Python artist; return *artist*.

    The path effects as given; the callable picker they hold, which the
    pick calls, and any other picker as given (``picker=5`` reads back as
    ``5``, as matplotlib's ``set_picker`` keeps it); the clip box and the
    clip path as `.Artist.set_clip_box` and `.Artist.set_clip_path` keep
    them, then clipped as matplotlib's add method leaves the artist
    (`clipped_as_added`).
    """
    if "path_effects" in kwargs:
        # As `.Artist.set_path_effects` keeps them, the very list given.
        artist.__dict__["_path_effects"] = kwargs["path_effects"]
    picker = kwargs.get("picker")
    if callable(picker):
        artist.set_picker(picker)
    elif "picker" in kwargs:
        artist._keep_picker(picker)
    if "clip_box" in kwargs:
        artist.set_clip_box(kwargs["clip_box"])
    if kwargs.get("clip_path") is not None:
        artist.set_clip_path(kwargs["clip_path"])
    if "clip_box" in kwargs or "clip_path" in kwargs:
        clipped_as_added(artist, kwargs)
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
