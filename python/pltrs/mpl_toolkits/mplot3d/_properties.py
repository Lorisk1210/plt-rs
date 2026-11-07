"""
The properties the 3-D plotting methods give the artists they make.

matplotlib makes an artist with the properties given, and only then adds
it to the axes: a property no setter of its class takes fails the call
before anything is added. pltrs (Rust) makes the artist in the axes, so
the methods check the properties first (`check_known`).
"""

from pltrs import cbook


def check_known(cls, kwargs):
    """Raise the `AttributeError` matplotlib's ``set`` of an artist of
    *cls* (the class matplotlib makes it of, a 2-D one for the artists it
    lifts into 3-D later) raises for the first property of *kwargs* none of
    its setters takes."""
    for name in cbook.normalize_kwargs(kwargs, cls):
        if not callable(getattr(cls, f"set_{name}", None)):
            raise AttributeError(f"{cls.__name__}.set() got an unexpected "
                                 f"keyword argument {name!r}", name=name)
