"""
What ticking an axis leaves in its tick helpers, for the helpers pltrs
(Rust) ticks with natively.

matplotlib's ``Axis._update_ticks`` calls the locators and formatters of an
axis wherever it ticks it (drawing it, measuring it, reading its tick
labels), and they keep what they computed in plain attributes, which
programs read afterwards: a `.ScalarFormatter`'s ``get_offset()``,
``offset`` and order of magnitude, a `.ConciseDateFormatter`'s offset, the
frequency an `.AutoDateLocator` picked, the ``ndivs`` an
`.AutoMinorLocator` settled.

The helpers pltrs (Rust) ticks with itself (those it installed, and the
locators of ``ticker`` installed from their specs, see `._tick_helpers`)
are not called when it ticks their axis. Each computation of an axis' ticks
is numbered and keeps the locations its formatters labelled and its
description of the axis (``_native.axis.labelled``, ``labelling``). The
attributes matplotlib's update sets are `ticked` descriptors, which make
the helper of a natively ticked axis catch up with the axis' last
computation before they are read or set (`catch_up`): the helper does
quietly what matplotlib's update did with it, on the axis described as
pltrs (Rust) ticked it (``format_ticks(locs)`` for a formatter, a call for
a locator), once per computation.
"""

import contextlib
import warnings

from ._native import axis as _axis


class ticked:
    """
    An attribute of a tick helper that matplotlib's tick updates set: read
    or set, it first has the helper catch up with the last ticks of the
    axis pltrs (Rust) ticks with it (`catch_up`). It reads *default* until
    set.
    """

    def __init__(self, default=None):
        self._default = default

    def __set_name__(self, owner, name):
        self._name = name

    def __get__(self, helper, owner=None):
        if helper is None:
            return self._default
        catch_up(helper)
        return helper.__dict__.get(self._name, self._default)

    def __set__(self, helper, value):
        catch_up(helper)
        helper.__dict__[self._name] = value

    def __delete__(self, helper):
        catch_up(helper)
        try:
            del helper.__dict__[self._name]
        except KeyError:
            raise AttributeError(self._name) from None


def follow(helper, axis, helper_id):
    """Have *helper*, the object of the helper of identity *helper_id* that
    pltrs (Rust) ticks *axis* with natively, catch up with its ticks."""
    object.__setattr__(helper, "_ticked_on", (axis, helper_id))


def unfollow(helper):
    """Stop *helper* following the ticks of an axis (it is called when its
    axis is ticked from now on), after it caught up with them."""
    catch_up(helper)
    helper.__dict__.pop("_ticked_on", None)


def catch_up(helper):
    """
    Have *helper* do what matplotlib's helper did in the last computation
    of the ticks of the axis pltrs (Rust) ticks with it natively, if it
    has not yet: nothing for the others, for a helper no longer in use
    there, and while it catches up.
    """
    state = helper.__dict__
    ticked_on = state.get("_ticked_on")
    if ticked_on is None or state.get("_catching_up"):
        return
    axis, helper_id = ticked_on
    try:
        from ._tick_helpers import _axis_registry
        token, entry, _ = _axis_registry(axis)[helper_id]
        if entry is not helper:
            return
        found = _axis.labelled(*axis._args(), token)
        if found is None or state.get("_ticked_serial") == found[1]:
            return
        minor, serial = found
        labelling = _axis.labelling(*axis._args(), minor)
    except Exception:
        # The axes is gone, or its figure is being laid out or drawn.
        return
    object.__setattr__(helper, "_ticked_serial", serial)
    if labelling is not None:
        _replay(helper, axis, minor, *labelling)


def _replay(helper, axis, minor, locs, info):
    """
    Do with *helper* what matplotlib's ``_update_ticks`` does on the
    *minor* (or major) level of *axis*, as *info* describes it: label the
    locations *locs* with a formatter (as the level's locator gives them),
    call a locator. It warns and fails where the update did, so here it
    is quiet.
    """
    from ._tick_helpers import drawing, settled
    from .axis import _typed_locs
    from .ticker import Locator
    with _catching_up(helper), drawing(helper, info), warnings.catch_warnings():
        warnings.simplefilter("ignore")
        try:
            if isinstance(helper, Locator):
                helper()
            else:
                locator = (axis.get_minor_locator() if minor
                           else axis.get_major_locator())
                view = _axis.info_fields(info)[0]
                helper.format_ticks(_typed_locs(locator, locs, minor, view))
        except Exception:
            pass
    settled(helper)


@contextlib.contextmanager
def _catching_up(helper):
    """While *helper* catches up: it neither catches up again nor counts
    its attributes set as changes to install."""
    object.__setattr__(helper, "_catching_up", True)
    try:
        yield
    finally:
        object.__delattr__(helper, "_catching_up")


def catching_up(helper):
    """Whether *helper* is catching up with the ticks of its axis."""
    return bool(helper.__dict__.get("_catching_up"))
