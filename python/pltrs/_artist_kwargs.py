"""
The keyword arguments every artist takes that cross to pltrs (Rust) in a
form of their own: ``clip_path``, ``path_effects`` and ``picker``, as the
native property tables of lines, patches, collections and images take
them when an artist is made with them. A callable picker does not cross:
it stays with the Python artist (`keep_program_picker`), which the pick
calls, as `.Artist.set_picker` keeps it.
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
    callable picker they hold, if any; return *artist*."""
    picker = kwargs.get("picker")
    if callable(picker):
        artist.set_picker(picker)
    return artist
