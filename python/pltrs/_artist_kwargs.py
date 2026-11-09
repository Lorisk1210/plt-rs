"""
The keyword arguments every artist takes that cross to pltrs (Rust) in a
form of their own: ``clip_path``, ``path_effects`` and ``picker``, as the
native property tables of lines, patches, collections and images take
them when an artist is made with them. A callable picker does not cross:
it stays with the Python artist (`keep_program_picker`), which the pick
calls, as `.Artist.set_picker` keeps it. `check_known` refuses the
properties no setter takes, as matplotlib's constructors do.
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


def keep_program_picker(artist, kwargs):
    """Give *artist*, made with the keyword arguments *kwargs*, the
    callable picker they hold, if any, and keep any other picker as given
    (``picker=5`` reads back as ``5``, as matplotlib's ``set_picker`` keeps
    it); return *artist*."""
    picker = kwargs.get("picker")
    if callable(picker):
        artist.set_picker(picker)
    elif "picker" in kwargs:
        artist.__dict__["_picker"] = picker
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
