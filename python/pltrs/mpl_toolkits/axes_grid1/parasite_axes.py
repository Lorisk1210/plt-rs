"""
Host axes and their parasites (matplotlib's
``mpl_toolkits.axes_grid1.parasite_axes``, from matplotlib 3.11.2, under
the Matplotlib License): pltrs (Rust) makes the parasites on their host,
which draws them among its own artists and keeps their view limits in
step with its own (``_native.axes_grid1``).
"""

from pltrs import _api, _handles, cbook
import pltrs.artist as martist
import pltrs.axes as maxes
import pltrs.transforms as mtransforms
from pltrs._native import axes_grid1 as _axes_grid1
from .mpl_axes import Axes


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
        native = self._parasite_native(parent_axes, aux_transform,
                                       viewlim_mode)
        _handles.adopt(self, native, parent_axes.figure)
        self._init_axes_state()
        # The host draws the parasite's artists; what the parasite would
        # draw of its own stays hidden (matplotlib's `clear` hides its
        # children).
        for axis in getattr(self, "_axislines", {}).values():
            martist.Artist.set_visible(axis, False)
        for name in ("x", "y"):
            other = kwargs.pop(f"share{name}", None)
            if other is not None:
                getattr(self, f"share{name}")(other)
        self._internal_update(kwargs)

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

    def get_axes_locator(self):
        return self._parent_axes.get_axes_locator()

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


parasite_axes_class_factory = cbook._make_class_factory(
    ParasiteAxesBase, "{}Parasite")
ParasiteAxes = parasite_axes_class_factory(Axes)


class HostAxesBase:

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    @property
    def parasites(self):
        """The parasite axes of the host, in creation order."""
        return _handles.wrap_all(_axes_grid1.parasites(self._native),
                                 ParasiteAxes, self.figure)

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
        for ax in self.parasites:
            ax.apply_aspect(rect)

        super().draw(renderer)

    def clear(self):
        super().clear()
        for ax in self.parasites:
            ax.clear()

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
        native = _axes_grid1.twin(
            self._native, kind,
            None if aux_transform is None else _native_link(aux_transform,
                                                            viewlim_mode))
        return parasite_axes_class_factory(axes_class)._of_host(
            self, native, aux_transform, viewlim_mode)


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
