"""
What a draw of a host Axes that failed leaves matplotlib's host with.

matplotlib's ``HostAxesBase.draw`` lists the children of its parasites
among its own (``self._children.extend(ax.get_children())``) for the
``Axes.draw`` it calls, and takes them out once that draw returns: a draw
that fails leaves them listed. The host's next clear sets the Axes of all
it lists to None, the parasites' axes, spines and gridlines included,
which stay without their Axes for good (the parasites cannot be cleared
any more). Every later clear and draw of the host meets them:

* the host's clear gives the axes the parasites share with it a linear
  scale (``other._axis_map[name]._set_scale``), whose new locator, held
  by the ticker the host shares, belongs to the parasite's axis; then it
  sets its limits, telling the axes sharing them, and fails on a
  parasite's axis without its Axes (``'_axis_map'``);
* a clear getting past that clears the parasites, whose axes without
  their Axes fail to place their labels (``'transAxes'``);
* the host's draw fails on the first of them it draws (in z-order): an
  axis of its own ticked by a locator of such an axis, or such an axis
  reading its view (``'viewLim'``) or its own name (``'_axis_map'``), a
  spine reading the view, a gridline collection or an axis artist of
  axisartist updating its limits (``'get_xlim'``).

pltrs (Rust) keeps every part with its parasite (and makes some of them
anew as it clears it), so the host keeps the parasites whose parts lost
their Axes, and fails where matplotlib's does (`._unusable`).
"""

from pltrs import _unusable

#: The name under which the host keeps its draw failing (`._unusable`).
_KEY = "parasite parts without their Axes"


def leave(host, parasites):
    """Note that a draw of *host* failed once it listed the children of
    *parasites*."""
    host.__dict__.setdefault("_left_by_failed_draws", []).extend(parasites)


def clear(host, clear_host, clear_parasite):
    """
    Clear *host* (through *clear_host*) and its parasites (each through
    *clear_parasite*) as matplotlib's ``HostAxesBase.clear`` does with the
    parts failed draws left listed, failing where it fails.
    """
    state = host.__dict__
    orphans = state.setdefault("_orphaned_parasites", [])
    orphans.extend(parasite for parasite in state.pop("_left_by_failed_draws", [])
                   if not _among(parasite, orphans))
    owners = state.setdefault("_ticker_owners", {})
    clear_host()
    if not orphans:
        for parasite in host.parasites:
            clear_parasite(parasite)
        return
    for parasite in host.parasites:
        # pltrs (Rust) cleared the parasites with their host: those of
        # axisartist's make their axis artists and gridlines anew.
        remake = getattr(parasite, "_remake_axis_artists", None)
        if remake is not None:
            remake()
    try:
        _clear_host_limits(host, orphans, owners)
        for parasite in host.parasites:
            if _among(parasite, orphans):
                # Its axis' clear places the label with its Axes.
                getattr(None, "transAxes")
            clear_parasite(parasite)
            for name in parasite._axis_names:
                if getattr(parasite, f"_share{name}", None) is host:
                    owners.pop(name, None)
    finally:
        _unusable.keep(host, _KEY, None, lambda _: _fail_as_drawn(host), measured=False)


def _clear_host_limits(host, orphans, owners):
    """matplotlib's clear of *host* past its own axes' clear: the axes
    sharing its axes take a linear scale (the ticker they share then
    holding locators of theirs), then the host sets its limits, telling
    them."""
    for name in host._axis_names:
        for other in _siblings(host, name):
            if _among(other, orphans):
                owners[name] = other
    for name in host._axis_names:
        if getattr(host, f"_share{name}", None) is not None:
            continue
        if any(_among(other, orphans) for other in _siblings(host, name)):
            # The parasite's axis names itself by its Axes.
            getattr(None, "_axis_map")


def _siblings(host, name):
    """The Axes sharing the axis *name* of *host*, but the host."""
    return [other for other in host._shared_axes[name].get_siblings(host)
            if other is not host]


def _among(axes, group):
    """Whether *axes* is one of *group*."""
    return any(axes is other for other in group)


def _fail_as_drawn(host):
    """Fail as matplotlib's draw of *host* fails on the first of its
    children (in z-order) its orphaned parasites' parts make fail: those
    failed draws left listed, those this draw lists, then the host's own
    axes (its other children do not fail)."""
    orphans = host.__dict__.get("_orphaned_parasites", [])
    owners = host.__dict__.get("_ticker_owners", {})
    listed = [(parasite, part)
              for parasite in [*host.__dict__.get("_left_by_failed_draws", []),
                               *host.parasites]
              for part in parasite.get_children() if part is not None]
    listed += [(host, getattr(host, f"{name}axis")) for name in host._axis_names]
    for owner, part in sorted(listed, key=lambda item: item[1].get_zorder()):
        if _among(owner, orphans):
            _fail_lost(part)
        elif owner is host and any(part is getattr(host, f"{name}axis") for name in owners):
            _fail_ticking(part)


def _fail_lost(part):
    """Fail as matplotlib's draw of *part*, without its Axes, fails."""
    from pltrs.axis import Axis
    from pltrs.spines import Spine
    from ..axisartist.axis_artist import AxisArtist, GridlinesCollection
    if isinstance(part, GridlinesCollection):
        getattr(None, "get_xlim")
    if not part.get_visible():
        return
    if isinstance(part, AxisArtist):
        getattr(None, "get_xlim")
    if isinstance(part, Spine):
        getattr(None, "viewLim")
    if isinstance(part, Axis):
        _fail_ticking(part)
        getattr(None, "_axis_map")


def _fail_ticking(axis):
    """Fail as matplotlib's draw of *axis* fails ticking through a locator
    of an axis without its Axes (or its own view): when it is visible and
    places ticks."""
    from pltrs.ticker import NullLocator
    kw = axis._major_tick_kw
    ticked = any(kw.get(key) is not False
                 for key in ("tick1On", "tick2On", "label1On", "label2On", "gridOn"))
    if (axis.get_visible() and ticked
            and not isinstance(axis.get_major_locator(), NullLocator)):
        getattr(None, "viewLim")
