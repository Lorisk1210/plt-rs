"""
Host axes and their parasites (matplotlib's
``mpl_toolkits.axes_grid1.parasite_axes``, from matplotlib 3.11.2, under
the Matplotlib License): pltrs (Rust) makes the parasites on their host,
which draws them among its own artists and keeps their view limits in
step with its own (``_native.axes_grid1``). A parasite drawn over its host
keeps the position it is given for its getters, as matplotlib's keeps one
of its own that only its host's draw replaces.
"""

from pltrs import _api, _docstring, _handles, cbook
import pltrs.artist as martist
import pltrs.axes as maxes
import pltrs.transforms as mtransforms
from pltrs._native import axes_grid1 as _axes_grid1
from . import _failed_draws
from .mpl_axes import Axes


def _check_composes(aux_transform, parent_axes):
    """Raise what matplotlib's ``_set_lim_and_transforms`` raises composing
    *aux_transform* with the data transform of *parent_axes*
    (``transAux + parent.transData``) for what is no transform."""
    if aux_transform is not None:
        aux_transform + parent_axes.transData


def _native_link(aux_transform, viewlim_mode):
    """How pltrs (Rust) relates a parasite with *aux_transform* (affine)
    and *viewlim_mode* to its host: drawn through the aux transform, or
    with its own data coordinates, following the host's view limits
    (``"equal"``) or not."""
    if aux_transform is None:
        if viewlim_mode == "transform":
            raise ValueError("viewlim_mode='transform' needs an aux transform")
        return "independent" if viewlim_mode is None else "equal"
    return aux_transform.get_matrix()


class ParasiteAxesBase:

    def __init__(self, parent_axes, aux_transform=None,
                 *, viewlim_mode=None, **kwargs):
        self._parent_axes = parent_axes
        self.transAux = aux_transform
        self.set_viewlim_mode(viewlim_mode)
        kwargs.pop("frameon", None)
        if not isinstance(self, maxes.Axes):
            # Mixed into no Axes class: the next class's constructor makes
            # the object, as in matplotlib (object's refuses the arguments).
            kwargs["frameon"] = False
            super().__init__(parent_axes.get_figure(root=False),
                             parent_axes._position, **kwargs)
            return
        # What matplotlib's constructor reads first of the parent.
        parent_axes.get_figure(root=False)
        _check_composes(aux_transform, parent_axes)
        native = self._parasite_native(parent_axes, aux_transform,
                                       viewlim_mode)
        _handles.adopt(self, native, parent_axes.figure)
        self._init_axes_state()
        # The host draws the parasite's artists; what the parasite would
        # draw of its own stays hidden (matplotlib's `clear` hides its
        # children).
        for axis in getattr(self, "_axislines", {}).values():
            martist.Artist.set_visible(axis, False)
        # What matplotlib's constructor connects as it clears the parasite.
        self._follow_parent_lims()
        try:
            for name in ("x", "y"):
                other = kwargs.pop(f"share{name}", None)
                if other is not None:
                    getattr(self, f"share{name}")(other)
            self._internal_update(kwargs)
        except Exception:
            # matplotlib's host lists a parasite once it is made: one
            # whose constructor failed is in no host.
            _axes_grid1.remove_parasite(parent_axes._native, native)
            _handles.forget(self)
            raise

    def _parasite_native(self, parent_axes, aux_transform, viewlim_mode):
        """The native parasite of *parent_axes* this parasite is: one whose
        data a non-affine aux transform maps onto the host's data is drawn
        as an axisartist parasite (``get_aux_axes(tr)``) draws it."""
        native_aux = getattr(super(), "_aux_parasite_native", None)
        if native_aux is not None and aux_transform is not None:
            return native_aux(parent_axes, aux_transform, viewlim_mode)
        if aux_transform is not None and not aux_transform.is_affine:
            from pltrs._native import axisartist as _axisartist
            from ..axisartist.grid_helper_curvelinear import _native_aux
            return _axisartist.aux_axes(parent_axes._native,
                                        _native_aux(aux_transform))
        return _axes_grid1.aux_axes(parent_axes._native,
                                    _native_link(aux_transform, viewlim_mode))

    @classmethod
    def _of_host(cls, host, native, aux_transform, viewlim_mode):
        """The parasite object of class *cls* around the native parasite
        *native* pltrs (Rust) made on *host*."""
        ax = _handles.wrap(native, cls, host.figure)
        ax._parent_axes = host
        ax.transAux = aux_transform
        ax.set_viewlim_mode(viewlim_mode)
        ax._follow_parent_lims()
        return ax

    def _set_lim_and_transforms(self):
        # The trees of an Axes, then matplotlib's: with an aux transform,
        # the parasite's data coordinates go through it to the parent's.
        super()._set_lim_and_transforms()
        if self.transAux is not None:
            self.transAxes = self._parent_axes.transAxes
            self.transData = self.transAux + self._parent_axes.transData
            self._xaxis_transform = mtransforms.blended_transform_factory(
                self.transData, self.transAxes)
            self._yaxis_transform = mtransforms.blended_transform_factory(
                self.transAxes, self.transData)

    def clear(self):
        super().clear()
        for child in self.get_children():
            child.set_visible(False)
        self._follow_parent_lims()

    def _follow_parent_lims(self):
        """Connect `_sync_lims` to the limit changes of the parent, as
        matplotlib's ``clear`` does."""
        self._parent_axes.callbacks._connect_picklable(
            "xlim_changed", self._sync_lims)
        self._parent_axes.callbacks._connect_picklable(
            "ylim_changed", self._sync_lims)

    def get_axes_locator(self):
        return self._parent_axes.get_axes_locator()

    @_docstring.copy(maxes.Axes.set_axes_locator)
    def set_axes_locator(self, locator):
        # matplotlib keeps it, but never calls it: the parasite follows its
        # host's locator (`get_axes_locator`), and only its host is drawn.
        self._axes_locator = locator
        self.stale = True

    # matplotlib's parasite has a position of its own, its host's when it
    # was made, which its host's draw replaces by the host's box for that
    # draw (``ax.apply_aspect(rect)``). pltrs (Rust) draws a parasite over
    # its host whatever position it has, and measures its tight box at its
    # own original position; the active one is kept here, for the getters
    # to read back.

    def _own_positions(self):
        """The parasite's own original and active positions: both the
        original one pltrs (Rust) holds until one is given."""
        own = self.__dict__.get("_own_position")
        if own is None:
            given = mtransforms.Bbox.from_extents(
                *_axes_grid1.own_position(self._native))
            own = self._own_position = {"original": given, "active": given}
        return own

    def _set_position(self, pos, which='both'):
        if not isinstance(pos, mtransforms.BboxBase):
            pos = mtransforms.Bbox.from_bounds(*pos)
        own = self._own_positions()
        for kept in ("active", "original"):
            if which in ("both", kept):
                own[kept] = pos.frozen()
        if which in ("both", "original"):
            _axes_grid1.set_own_position(self._native,
                                         tuple(map(float, pos.bounds)))
        self.stale = True

    @_docstring.copy(maxes.Axes.set_position)
    def set_position(self, pos, which='both'):
        self._set_position(pos, which=which)
        # Because this is being called externally to the library we
        # don't let it be in the layout.
        self.set_in_layout(False)

    def get_position(self, original=False):
        # docstring inherited
        if not original and not self.get_axes_locator():
            # matplotlib's apply_aspect sets the active position anew.
            self.apply_aspect()
        return self._own_positions()[
            "original" if original else "active"].frozen()

    def apply_aspect(self, position=None):
        # docstring inherited
        if position is None:
            position = self._own_positions()["original"]
        position = mtransforms.Bbox.from_bounds(*position.bounds)
        aspect, box_aspect = self.get_aspect(), self.get_box_aspect()
        if self.get_adjustable() == "box" and aspect != "auto":
            box_aspect = aspect * self.get_data_ratio()
        elif box_aspect is None:
            # The view limits take the aspect ("datalim"), as pltrs (Rust)
            # has them take it in its host's box.
            super().apply_aspect()
            self._set_position(position, which="active")
            return
        # matplotlib's box made to the aspect in the given one.
        figure_box = mtransforms.Bbox.unit().transformed(
            self.get_figure(root=False).transSubfigure)
        shrunk = position.shrunk_to_aspect(
            box_aspect, position, figure_box.height / figure_box.width)
        self._set_position(shrunk.anchored(self.get_anchor(), position),
                           which="active")

    def get_subplotspec(self):
        # docstring inherited
        return self.__dict__.get("_own_subplotspec") or super().get_subplotspec()

    def set_subplotspec(self, subplotspec):
        # docstring inherited
        self._set_position(subplotspec.get_position(self.get_figure(root=False)))
        self._own_subplotspec = subplotspec

    def pick(self, mouseevent):
        # This most likely goes to Artist.pick (depending on axes_class given
        # to the factory), which only handles pick events registered on the
        # axes associated with each child:
        super().pick(mouseevent)
        # But parasite axes are additionally given pick events from their host
        # axes (cf. HostAxesBase.pick), which we handle here:
        for a in self.get_children():
            if (hasattr(mouseevent.inaxes, "parasites")
                    and self in mouseevent.inaxes.parasites):
                a.pick(mouseevent)

    def remove(self):
        """Remove the parasite from its host (for a twin, the host's far
        sides come back)."""
        _axes_grid1.remove_parasite(self._parent_axes._native, self._native)
        _handles.forget(self)

    def set_viewlim_mode(self, mode):
        _api.check_in_list([None, "equal", "transform"], mode=mode)
        self._viewlim_mode = mode

    def get_viewlim_mode(self):
        return self._viewlim_mode

    def _sync_lims(self, parent):
        """Have the view limits follow those of *parent* by the view
        limits mode, as matplotlib's callback of the parent's limit changes
        does: pltrs (Rust) keeps the limits of a parasite of mode "equal"
        or "transform" so, and those of one of no mode as they are."""
        parent.viewLim.frozen()
        _api.check_in_list([None, "equal", "transform"],
                           mode=self.get_viewlim_mode())


parasite_axes_class_factory = cbook._make_class_factory(
    ParasiteAxesBase, "{}Parasite")
ParasiteAxes = parasite_axes_class_factory(Axes)


class HostAxesBase:

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    def __init__(self, *args, **kwargs):
        # matplotlib's signature: the host's Axes class takes the arguments
        # (pltrs (Rust) keeps the parasites, see `parasites`).
        super().__init__(*args, **kwargs)

    @property
    def parasites(self):
        """The parasite axes of the host, in creation order."""
        return _handles.wrap_all(_axes_grid1.parasites(self._native),
                                 ParasiteAxes, self.figure)

    def set_in_layout(self, in_layout):
        # docstring inherited
        super().set_in_layout(in_layout)
        self._in_layout_set_at = martist._artist.draw_count(self._native)

    def get_in_layout(self):
        # docstring inherited
        # matplotlib's draw of a host placed by a locator gives it its
        # place with `set_position`, which takes it out of the layout:
        # pltrs (Rust) draws the host, so the draws since the flag was set
        # tell.
        drawn = martist._artist.draw_count(self._native)
        if (drawn > self.__dict__.get("_in_layout_set_at", 0)
                and self.get_axes_locator() and super().get_in_layout()):
            self.set_in_layout(False)
        return super().get_in_layout()

    def get_aux_axes(
            self, tr=None, viewlim_mode="equal", axes_class=None, **kwargs):
        """
        Add a parasite axes to this host.

        Despite this method's name, this should actually be thought of as an
        ``add_parasite_axes`` method.

        .. versionchanged:: 3.7
           Defaults to same base axes class as host axes.

        Parameters
        ----------
        tr : `~matplotlib.transforms.Transform` or None, default: None
            If a `.Transform`, the following relation will hold:
            ``parasite.transData = tr + host.transData``.
            If None, the parasite's and the host's ``transData`` are unrelated.
        viewlim_mode : {"equal", "transform", None}, default: "equal"
            How the parasite's view limits are set: directly equal to the
            parent axes ("equal"), equal after application of *tr*
            ("transform"), or independently (None).
        axes_class : subclass type of `~matplotlib.axes.Axes`, optional
            The `~.axes.Axes` subclass that is instantiated.  If None, the base
            class of the host axes is used.
        **kwargs
            Other parameters are forwarded to the parasite axes constructor.
        """
        if axes_class is None:
            axes_class = self._base_axes_class
        parasite_axes_class = parasite_axes_class_factory(axes_class)
        ax2 = parasite_axes_class(
            self, tr, viewlim_mode=viewlim_mode, **kwargs)
        # note that ax2.transData == tr + ax1.transData
        # Anything you draw in ax2 will match the ticks and grids of ax1.
        return ax2

    def draw(self, renderer):
        # Draw the host and the artists of its parasites, placed by the host's
        # locator and aspect (pltrs (Rust) draws the parasites' artists among
        # the host's).
        locator = self.get_axes_locator()
        if locator:
            pos = locator(self, renderer)
            self.set_position(pos, which="active")
            self.apply_aspect(pos)
        else:
            self.apply_aspect()

        rect = self.get_position()
        # matplotlib lists the parasites' children among the host's for its
        # draw, which a draw that fails leaves listed.
        listed = []
        try:
            for ax in self.parasites:
                ax.apply_aspect(rect)
                listed.append(ax)
            super().draw(renderer)
        except Exception:
            _failed_draws.leave(self, listed)
            raise

    def _filter_failed(self):
        # Its agg filter fails matplotlib's Axes draw, the parasites'
        # children listed (pltrs (Rust) draws the figure).
        _failed_draws.leave(self, list(self.parasites))

    def clear(self):
        _failed_draws.clear(self, super().clear, lambda ax: ax.clear())

    def get_tightbbox(self, renderer=None, *, call_axes_locator=True,
                      bbox_extra_artists=None):
        bbs = [
            *[ax.get_tightbbox(renderer, call_axes_locator=call_axes_locator)
              for ax in self.parasites],
            super().get_tightbbox(renderer,
                                  call_axes_locator=call_axes_locator,
                                  bbox_extra_artists=bbox_extra_artists)]
        return mtransforms.Bbox.union(
            [b for b in bbs if b is not None and (b.width != 0 or b.height != 0)])

    def pick(self, mouseevent):
        super().pick(mouseevent)
        # Also pass pick events on to parasite axes and, in turn, their
        # children (cf. ParasiteAxesBase.pick)
        for a in self.parasites:
            a.pick(mouseevent)

    def twinx(self, axes_class=None):
        """
        Create a twin of Axes with a shared x-axis but independent y-axis.

        The y-axis of self will have ticks on the left and the returned axes
        will have ticks on the right.
        """
        ax = self._add_twin_axes(axes_class, "x")
        self.axis["right"].set_visible(False)
        ax.axis["right"].set_visible(True)
        ax.axis["left", "top", "bottom"].set_visible(False)
        return ax

    def twiny(self, axes_class=None):
        """
        Create a twin of Axes with a shared y-axis but independent x-axis.

        The x-axis of self will have ticks on the bottom and the returned axes
        will have ticks on the top.
        """
        ax = self._add_twin_axes(axes_class, "y")
        self.axis["top"].set_visible(False)
        ax.axis["top"].set_visible(True)
        ax.axis["left", "right", "bottom"].set_visible(False)
        return ax

    def twin(self, aux_trans=None, axes_class=None):
        """
        Create a twin of Axes with no shared axis.

        While self will have ticks on the left and bottom axis, the returned
        axes will have ticks on the top and right axis.
        """
        if aux_trans is None:
            aux_trans = mtransforms.IdentityTransform()
        ax = self._add_twin_axes(axes_class, "both", aux_trans, "transform")
        self.axis["top", "right"].set_visible(False)
        ax.axis["top", "right"].set_visible(True)
        ax.axis["left", "bottom"].set_visible(False)
        return ax

    def _add_twin_axes(self, axes_class, kind, aux_transform=None,
                       viewlim_mode=None):
        """
        Helper for `.twinx`/`.twiny`/`.twin`: pltrs (Rust) makes the twin
        sharing the *kind* axis (``"x"``, ``"y"``, or ``"both"`` for none,
        through *aux_transform*) and shows its sides opposite the host's.
        """
        if axes_class is None:
            axes_class = self._base_axes_class
        parasite_class = parasite_axes_class_factory(axes_class)
        _check_composes(aux_transform, self)
        native = _axes_grid1.twin(
            self._native, kind,
            None if aux_transform is None else _native_link(aux_transform,
                                                            viewlim_mode))
        return parasite_class._of_host(self, native, aux_transform, viewlim_mode)


# The methods matplotlib's hosts inherit.
cbook._inherited_methods(HostAxesBase, martist.Artist, ["set_in_layout", "get_in_layout"])

host_axes_class_factory = host_subplot_class_factory = \
    cbook._make_class_factory(HostAxesBase, "{}HostAxes", "_base_axes_class")
HostAxes = SubplotHost = host_axes_class_factory(Axes)


def host_axes(*args, axes_class=Axes, figure=None, **kwargs):
    """
    Create axes that can act as a hosts to parasitic axes.

    Parameters
    ----------
    figure : `~matplotlib.figure.Figure`
        Figure to which the axes will be added. Defaults to the current figure
        `.pyplot.gcf()`.

    *args, **kwargs
        Will be passed on to the underlying `~.axes.Axes` object creation.
    """
    import pltrs.pyplot as plt
    host_axes_class = host_axes_class_factory(axes_class)
    if figure is None:
        figure = plt.gcf()
    ax = host_axes_class(figure, *args, **kwargs)
    figure.add_axes(ax)
    return ax


host_subplot = host_axes


# What matplotlib's module also has from its imports, which programs
# import from it.
from pltrs.transforms import Bbox  # noqa: E402, F401
