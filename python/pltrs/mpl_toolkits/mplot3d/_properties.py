"""
The properties the 3-D plotting methods give the artists they make.

matplotlib makes an artist with the properties given, and only then adds
it to the axes: a property no setter of its class takes fails the call
before anything is added. pltrs (Rust) makes the artist in the axes, so
the methods check the properties first (`check_known`).
"""

from pltrs._artist_kwargs import check_known  # noqa: F401  (the methods' name for it)
