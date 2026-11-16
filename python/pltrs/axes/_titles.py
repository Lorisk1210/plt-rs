"""
The titles of an Axes (matplotlib's ``set_title``, ``get_title``,
``title``, ``_left_title``, ``_right_title``, ``titleOffsetTrans`` and
``_update_title_position``).

pltrs (Rust) keeps the three titles of each axes and lifts them above
the decorations on top of it as it lays the figure out, unless the Axes
class places them its own way (its ``_update_title_position``): pltrs
(Rust) then leaves them where it puts them, and calls it where
matplotlib's ``draw`` and ``get_tightbbox`` do. The argument
handling of ``set_title`` and ``get_title`` is matplotlib 3.11.2's,
under the Matplotlib License.
"""

from numbers import Real

from .. import _api, _handles, _live_transforms, _unusable, _values
from .._native import axes as _axes
from .._program_classes import takes_from_program
from ..text import Text
from ..transforms import ScaledTranslation

#: The title locations, as ``loc`` names them.
_LOCATIONS = {"left": "left", "center": "center", "right": "right"}


class TitlesMixin:
    """The title methods of `~.axes.Axes`."""

    def _title_at(self, loc):
        return _handles.wrap(_axes.title_text(self._native, loc), Text,
                             self._figure)

    @_values.reassignable
    def title(self):
        """The centre title (a `.Text`)."""
        return self._title_at("center")

    @property
    def _left_title(self):
        return self._title_at("left")

    @property
    def _right_title(self):
        return self._title_at("right")

    @property
    def titleOffsetTrans(self):
        """The offset of the titles above the top of the Axes, in points of
        the figure (a `.ScaledTranslation`)."""
        pad = _axes.get_title_pad(self._native)
        # One object while the pad stays (matplotlib makes a new one when
        # ``set_title`` is given a pad).
        return _live_transforms.kept(
            self, "titleOffsetTrans", pad,
            lambda: ScaledTranslation(0.0, pad / 72,
                                      self.get_figure(root=False).dpi_scale_trans))

    def _set_title_offset_trans(self, title_offset_points):
        """
        Set the offset for the title either from :rc:`axes.titlepad`
        or from set_title kwarg ``pad``.
        """
        _axes.set_title_pad(self._native, float(title_offset_points))
        for title in (self.title, self._left_title, self._right_title):
            title.set_clip_box(None)

    def set_title(self, label, fontdict=None, loc=None, pad=None, *, y=None,
                  **kwargs):
        """
        Set a title for the Axes.

        Set one of the three available Axes titles. The available titles
        are positioned above the Axes in the center, flush with the left
        edge, and flush with the right edge.

        Parameters
        ----------
        label : str
            Text to use for the title

        fontdict : dict

            .. admonition:: Discouraged

               The use of *fontdict* is discouraged. Parameters should be passed as
               individual keyword arguments or using dictionary-unpacking
               ``set_title(..., **fontdict)``.

            A dictionary controlling the appearance of the title text,
            the default *fontdict* is::

               {'fontsize': rcParams['axes.titlesize'],
                'fontweight': rcParams['axes.titleweight'],
                'color': rcParams['axes.titlecolor'],
                'verticalalignment': 'baseline',
                'horizontalalignment': loc}

        loc : {'center', 'left', 'right'}, default: :rc:`axes.titlelocation`
            Which title to set.

        y : float, default: :rc:`axes.titley`
            Vertical Axes location for the title (1.0 is the top).  If
            None (the default) and :rc:`axes.titley` is also None, y is
            determined automatically to avoid decorators on the Axes.

        pad : float, default: :rc:`axes.titlepad`
            The offset of the title from the top of the Axes, in points.

        Returns
        -------
        `.Text`
            The matplotlib text instance representing the title

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            Other keyword arguments are text properties, see `.Text` for a list
            of valid text properties.
        """
        if loc is None:
            from .. import rcParams
            loc = rcParams['axes.titlelocation']
        loc = _api.getitem_checked(_LOCATIONS, loc=loc.lower())
        # A height that is no number the title keeps as given (matplotlib's
        # `.Text.set_y`), failing as its draws convert it.
        kept_y = y is not None and not isinstance(y, Real)
        title = _handles.wrap(
            _axes.set_title(self._native, loc, "",
                            None if pad is None else float(pad),
                            None if y is None or kept_y else float(y)),
            Text, self._figure)
        title.set_text(label)
        if "update" in title.__dict__:
            # The program gave the title an `update` of its own
            # (ultraplot's labels), which matplotlib's `set_title` calls
            # with the title's defaults.
            title.update(_title_defaults(loc))
        if fontdict is not None:
            title.update(fontdict)
        title._internal_update(kwargs)
        if kept_y:
            self._autotitlepos = False
            title.set_y(y)
        elif _unusable.kept(title, "y", None) is not None:
            # The height replaces the one kept, as matplotlib sets it anew.
            if y is None:
                from .. import rcParams
                y = rcParams['axes.titley']
            title.set_y(1.0 if y is None else y)
        return title

    def get_title(self, loc="center"):
        """
        Get an Axes title.

        Get one of the three available Axes titles. The available titles
        are positioned above the Axes in the center, flush with the left
        edge, and flush with the right edge.

        Parameters
        ----------
        loc : {'center', 'left', 'right'}, str, default: 'center'
            Which title to return.

        Returns
        -------
        str
            The title text string.

        """
        loc = _api.getitem_checked(_LOCATIONS, loc=loc.lower())
        return self._title_at(loc).get_text()

    def _update_title_position(self, renderer):
        """
        Update the title position based on the bounding box enclosing
        all the ticklabels and x-axis spine and xlabel...
        """
        _axes.update_title_position(self._native)

    @property
    def _autotitlepos(self):
        # Whether the titles are lifted above the decorations on top
        # (matplotlib's attribute, False once ``axes.titley`` or
        # ``set_title(y=...)`` fixed their height).
        return _axes.title_placement(self._native) != "fixed"

    @_autotitlepos.setter
    def _autotitlepos(self, auto):
        _axes.set_title_placement(
            self._native,
            ("fixed" if not auto
             else "program" if self._places_titles() else "auto"))

    def _places_titles(self):
        """Whether the class places the titles its own way (its
        ``_update_title_position``), which pltrs (Rust) then leaves to it
        (`_place_titles_by_program`)."""
        return takes_from_program(type(self), "_update_title_position")

    def _place_titles_by_program(self, renderer=None):
        """Place the titles as the class does, where matplotlib's
        ``draw`` and ``get_tightbbox`` call its ``_update_title_position``
        (cartopy's ``GeoAxes`` lifts them above its gridliner's labels)."""
        if self._places_titles():
            if renderer is None:
                # That of the print running, as matplotlib's draw hands it.
                from .._python_drawing import drawing_renderer
                renderer = drawing_renderer(self)
            self._update_title_position(renderer)

    def _readies_before_measured(self):
        """Whether the Axes places its titles its own way before pltrs
        (Rust) measures it for a layout or a tight box."""
        return self._places_titles()

    def _before_measured(self):
        """Apply the aspect and place the titles as the class does, where
        matplotlib's ``get_tightbbox`` does (mpltern's ``TernaryAxes``
        lifts them above the tick labels of its three axes, which a layout
        then makes room for)."""
        from .._python_drawing import _measures
        if _measures(type(self)):
            # The class measures it itself: its ``get_tightbbox`` does both
            # (through matplotlib's, as `.Axes.get_tightbbox` does).
            return False
        self.apply_aspect()
        self._place_titles_by_program()
        # Measured by pltrs (Rust).
        return True

    #: What a class of the program's may compute that matplotlib's ``draw``
    #: reads: the titles' place, which it places when drawn
    #: (`_push_computed`).
    _computed = ("_update_title_position",)

    def _push_computed(self):
        """Place the titles as the class does, as matplotlib's ``draw``
        does before it draws the Axes' artists."""
        self._place_titles_by_program()


def _title_defaults(loc):
    """The properties matplotlib's `set_title` updates a title with first
    (which pltrs (Rust) gives it itself)."""
    from .. import cbook, rcParams
    default = {
        'fontsize': rcParams['axes.titlesize'],
        'fontweight': rcParams['axes.titleweight'],
        'verticalalignment': 'baseline',
        'horizontalalignment': loc}
    titlecolor = rcParams['axes.titlecolor']
    if not cbook._str_lower_equal(titlecolor, 'auto'):
        default["color"] = titlecolor
    return default
