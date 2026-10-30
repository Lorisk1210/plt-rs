"""
Picking and the data under the pointer, for every artist (matplotlib's
``Artist.set_picker``, ``contains``, ``pick``, ``get_cursor_data`` and
``format_cursor_data``).

pltrs (Rust) keeps the pickers it decides with (``True`` and a
tolerance), hit-tests artists in the figure's current layout and orders a
pick as matplotlib's recursion over the children does. A callable picker
stays with the Python artist, which `PickingMixin.pick` asks itself.
"""

from numbers import Number

import numpy as np

from ._native import artist as _artist


class PickingMixin:
    """The picking and cursor data methods of `.Artist`."""

    def _different_canvas(self, event):
        """
        Check whether an *event* occurred on a canvas other that this artist's
        canvas.

        If this method returns True, the event definitely occurred on a
        different canvas; if it returns False, either it occurred on the same
        canvas, or we may not have enough information to know.
        """
        return (getattr(event, "canvas", None) is not None
                and (fig := self.get_figure(root=True)) is not None
                and event.canvas is not fig.canvas)

    def _pick_radius(self):
        """The tolerance `contains` hit-tests with: a number given as the
        picker, else None for the artist's own (its pick radius)."""
        picker = self.get_picker()
        if isinstance(picker, Number) and not isinstance(picker, bool):
            return float(picker)
        return None

    def _contains_details(self, ind):
        """The details `contains` gives of a hit on the items *ind*: none,
        for an artist hit as a whole."""
        return {}

    def contains(self, mouseevent):
        """
        Test whether the artist contains the mouse event.

        Parameters
        ----------
        mouseevent : `~matplotlib.backend_bases.MouseEvent`

        Returns
        -------
        contains : bool
            Whether any values are within the radius.
        details : dict
            An artist-specific dictionary of details of the event context,
            such as which points are contained in the pick radius.
        """
        if (self._different_canvas(mouseevent) or self._figure is None
                or mouseevent.x is None or mouseevent.y is None):
            return False, {}
        ind = _artist.contains(self._native, float(mouseevent.x),
                               float(mouseevent.y), self._pick_radius())
        if ind is None:
            return False, {}
        return True, self._contains_details(np.asarray(ind, dtype=int))

    def pickable(self):
        """
        Return whether the artist is pickable.

        See Also
        --------
        .Artist.set_picker, .Artist.get_picker, .Artist.pick
        """
        return (self.get_figure(root=False) is not None
                and self.get_picker() is not None)

    def pick(self, mouseevent):
        """
        Process a pick event.

        Each child artist will fire a pick event if *mouseevent* is over
        the artist and the artist has picker set.

        See Also
        --------
        .Artist.set_picker, .Artist.get_picker, .Artist.pickable
        """
        from .backend_bases import PickEvent
        figure = self.get_figure(root=True)
        if figure is None:
            return
        inaxes = getattr(mouseevent, "inaxes", None)
        asking = [artist._native for artist in list(figure._artists.values())
                  if callable(artist.__dict__.get("_picker"))]
        order = _artist.pick_order(
            self._native, None if inaxes is None else inaxes._native, asking)
        for native in order:
            artist = figure._artists.get(_artist.id(native))
            if artist is None:
                # Picked natively, the artist has no object to report.
                continue
            picker = artist.get_picker()
            if callable(picker):
                inside, prop = picker(artist, mouseevent)
            else:
                inside, prop = artist.contains(mouseevent)
            if inside:
                PickEvent("pick_event", figure.canvas, mouseevent, artist,
                          **prop)._process()

    def set_picker(self, picker):
        """
        Define the picking behavior of the artist.

        Parameters
        ----------
        picker : None or bool or float or callable
            This can be one of the following:

            - *None*: Picking is disabled for this artist (default).

            - A boolean: If *True* then picking will be enabled and the
              artist will fire a pick event if the mouse event is over
              the artist.

            - A float: If picker is a number it is interpreted as an
              epsilon tolerance in points and the artist will fire
              off an event if its data is within epsilon of the mouse
              event.

            - A function: If picker is callable, it is a user supplied
              function which determines whether the artist is hit by the
              mouse event::

                hit, props = picker(artist, mouseevent)

              to determine the hit test.  if the mouse event is over the
              artist, return *hit=True* and props is a dictionary of
              properties you want added to the PickEvent attributes.
        """
        self._picker = picker
        _artist.set_picker(self._native, None if callable(picker) else picker)

    def get_picker(self):
        """
        Return the picking behavior of the artist.

        The possible values are described in `.Artist.set_picker`.

        See Also
        --------
        .Artist.set_picker, .Artist.pickable, .Artist.pick
        """
        if "_picker" in self.__dict__:
            return self._picker
        return _artist.get_picker(self._native)

    def get_cursor_data(self, event):
        """
        Return the cursor data for a given event.

        Cursor data can be used by Artists to provide additional context
        information for a given event. The default implementation just returns
        *None*.

        Parameters
        ----------
        event : `~matplotlib.backend_bases.MouseEvent`
        """
        return None

    def format_cursor_data(self, data):
        """
        Return a string representation of *data*.

        The default implementation converts ints and floats and arrays of ints
        and floats into a comma-separated string enclosed in square brackets,
        unless the artist has an associated colorbar, in which case scalar
        values are formatted using the colorbar's formatter.
        """
        if np.ndim(data) == 0 and hasattr(self, "_format_cursor_data_override"):
            return self._format_cursor_data_override(data)
        try:
            data[0]
        except (TypeError, IndexError):
            data = [data]
        data_str = ', '.join(f'{item:0.3g}' for item in data
                             if isinstance(item, Number))
        return "[" + data_str + "]"
