"""
The arithmetic matplotlib's `.Quiver` does with its settings when it is
drawn, for arrows of settings pltrs (Rust) does not hold.

matplotlib's constructor keeps the scale, the sizes of the arrows, their
units and angles as given, and computes with them where it draws them
(``_init``, ``_make_verts``, ``_h_arrows``): a setting that is no number
or no unit it knows fails there, with the error Python's or numpy's
arithmetic raises, and numbers that are not finite warn as numpy does.
pltrs (Rust) draws the arrows of the settings it holds; a quiver keeping
others (see `._unusable`) runs this arithmetic where it is drawn, on a
copy of what it keeps (`Arrows`). Ported from matplotlib 3.11.2's
``quiver.py`` under the Matplotlib License.
"""

import math

import numpy as np
from numpy import ma

from . import _api, transforms


def draw_arrows(quiver, settings):
    """Compute the arrows of *quiver* with its *settings* (the scale,
    width, sizes, units and angles it was given) as matplotlib's draw
    does, failing and warning as it does."""
    arrows = Arrows(quiver, settings)
    arrows._init()
    arrows._make_verts(arrows.XY, arrows.U, arrows.V, arrows.angles)


class Arrows:
    """What matplotlib's `.Quiver` computes its arrows with: the settings
    given, the positions and components, its Axes."""

    def __init__(self, quiver, settings):
        self.__dict__.update(settings)
        self.axes = quiver.axes
        self.XY, self.U, self.V = quiver.XY, quiver._U, quiver._V
        self.N, self.Umask = quiver.N, quiver.Umask
        self.pivot = quiver.pivot
        self._dpi_at_last_init = None

    def _init(self):
        """
        Initialization delayed until first draw;
        allow time for axes setup.
        """
        trans = self._set_transform()
        self.span = trans.inverted().transform_bbox(self.axes.bbox).width
        if self.width is None:
            sn = np.clip(math.sqrt(self.N), 8, 25)
            self.width = 0.06 * self.span / sn

        # _make_verts sets self.scale if not already specified
        if (self._dpi_at_last_init != self.axes.get_figure(root=True).dpi
                and self.scale is None):
            self._make_verts(self.XY, self.U, self.V, self.angles)

        self._dpi_at_last_init = self.axes.get_figure(root=True).dpi

    def _dots_per_unit(self, units):
        """Return a scale factor for converting from units to pixels."""
        bb = self.axes.bbox
        vl = self.axes.viewLim
        return _api.getitem_checked({
            'x': bb.width / vl.width,
            'y': bb.height / vl.height,
            'xy': np.hypot(*bb.size) / np.hypot(*vl.size),
            'width': bb.width,
            'height': bb.height,
            'dots': 1.,
            'inches': self.axes.get_figure(root=True).dpi,
        }, units=units)

    def _set_transform(self):
        """
        The transform from arrow width units to pixels.
        """
        dx = self._dots_per_unit(self.units)
        self._trans_scale = dx  # pixels per arrow width unit
        return transforms.Affine2D().scale(dx)

    # Calculate angles and lengths for segment between (x, y), (x+u, y+v)
    def _angles_lengths(self, XY, U, V, eps=1):
        xy = self.axes.transData.transform(XY)
        uv = np.column_stack((U, V))
        xyp = self.axes.transData.transform(XY + eps * uv)
        dxy = xyp - xy
        angles = np.arctan2(dxy[:, 1], dxy[:, 0])
        lengths = np.hypot(*dxy.T) / eps
        return angles, lengths

    # XY is stacked [X, Y].
    # See quiver() doc for meaning of X, Y, U, V, angles.
    def _make_verts(self, XY, U, V, angles):
        uv = (U + V * 1j)
        str_angles = angles if isinstance(angles, str) else ''
        if str_angles == 'xy' and self.scale_units == 'xy':
            # Here eps is 1 so that if we get U, V by diffing
            # the X, Y arrays, the vectors will connect the
            # points, regardless of the axis scaling (including log).
            angles, lengths = self._angles_lengths(XY, U, V, eps=1)
        elif str_angles == 'xy' or self.scale_units == 'xy':
            # Calculate eps based on the extents of the plot
            # so that we don't end up with roundoff error from
            # adding a small number to a large.
            eps = np.abs(self.axes.dataLim.extents).max() * 0.001
            angles, lengths = self._angles_lengths(XY, U, V, eps=eps)

        if str_angles and self.scale_units == 'xy':
            a = lengths
        else:
            a = np.abs(uv)

        if self.scale is None:
            sn = max(10, math.sqrt(self.N))
            if self.Umask is not ma.nomask:
                amean = a[~self.Umask].mean()
            else:
                amean = a.mean()
            # crude auto-scaling
            # scale is typical arrow length as a multiple of the arrow width
            scale = 1.8 * amean * sn / self.span

        if self.scale_units is None:
            if self.scale is None:
                self.scale = scale
            widthu_per_lenu = 1.0
        else:
            if self.scale_units == 'xy':
                dx = 1
            else:
                dx = self._dots_per_unit(self.scale_units)
            widthu_per_lenu = dx / self._trans_scale
            if self.scale is None:
                self.scale = scale * widthu_per_lenu
        length = a * (widthu_per_lenu / (self.scale * self.width))
        X, Y = self._h_arrows(length)
        if str_angles == 'xy':
            theta = angles
        elif str_angles == 'uv':
            theta = np.angle(uv)
        else:
            theta = ma.masked_invalid(np.deg2rad(angles)).filled(0)
        theta = theta.reshape((-1, 1))  # for broadcasting
        xy = (X + Y * 1j) * np.exp(1j * theta) * self.width
        XY = np.stack((xy.real, xy.imag), axis=2)
        if self.Umask is not ma.nomask:
            XY = ma.array(XY)
            XY[self.Umask] = ma.masked
            # This might be handled more efficiently with nans, given
            # that nans will end up in the paths anyway.

        return XY

    def _h_arrows(self, length):
        """Length is in arrow width units."""
        # It might be possible to streamline the code
        # and speed it up a bit by using complex (x, y)
        # instead of separate arrays; but any gain would be slight.
        minsh = self.minshaft * self.headlength
        N = len(length)
        length = length.reshape(N, 1)
        # This number is chosen based on when pixel values overflow in Agg
        # causing rendering errors
        # length = np.minimum(length, 2 ** 16)
        np.clip(length, 0, 2 ** 16, out=length)
        # x, y: normal horizontal arrow
        x = np.array([0, -self.headaxislength,
                      -self.headlength, 0],
                     np.float64)
        x = x + np.array([0, 1, 1, 1]) * length
        y = 0.5 * np.array([1, 1, self.headwidth, 0], np.float64)
        y = np.repeat(y[np.newaxis, :], N, axis=0)
        # x0, y0: arrow without shaft, for short vectors
        x0 = np.array([0, minsh - self.headaxislength,
                       minsh - self.headlength, minsh], np.float64)
        y0 = 0.5 * np.array([1, 1, self.headwidth, 0], np.float64)
        ii = [0, 1, 2, 3, 2, 1, 0, 0]
        X = x[:, ii]
        Y = y[:, ii]
        Y[:, 3:-1] *= -1
        X0 = x0[ii]
        Y0 = y0[ii]
        Y0[3:-1] *= -1
        shrink = length / minsh if minsh != 0. else 0.
        X0 = shrink * X0[np.newaxis, :]
        Y0 = shrink * Y0[np.newaxis, :]
        short = np.repeat(length < minsh, 8, axis=1)
        # Now select X0, Y0 if short, otherwise X, Y
        np.copyto(X, X0, where=short)
        np.copyto(Y, Y0, where=short)
        if self.pivot == 'middle':
            X -= 0.5 * X[:, 3, np.newaxis]
        elif self.pivot == 'tip':
            # numpy bug? using -= does not work here unless we multiply by a
            # float first, as with 'mid'.
            X = X - X[:, 3, np.newaxis]
        elif self.pivot != 'tail':
            _api.check_in_list(["middle", "tip", "tail"], pivot=self.pivot)

        tooshort = length < self.minlength
        if tooshort.any():
            # Use a heptagonal dot:
            th = np.arange(0, 8, 1, np.float64) * (np.pi / 3.0)
            x1 = np.cos(th) * self.minlength * 0.5
            y1 = np.sin(th) * self.minlength * 0.5
            X1 = np.repeat(x1[np.newaxis, :], N, axis=0)
            Y1 = np.repeat(y1[np.newaxis, :], N, axis=0)
            tooshort = np.repeat(tooshort, 8, 1)
            np.copyto(X, X1, where=tooshort)
            np.copyto(Y, Y1, where=tooshort)
        # Mask handling is deferred to the caller, _make_verts.
        return X, Y
