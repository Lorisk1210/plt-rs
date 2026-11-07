"""
The wxPython backend drawing with Agg (matplotlib's ``backend_wxagg``): each
draw copies pltrs (Rust)'s raster canvas into the widget's bitmap.

From matplotlib 3.11.2, under the Matplotlib License, kept as it is but for
its imports of the package, and for showing what a draw or blit drew from
wx's main thread, whatever thread draws
(``_FigureCanvasWxBase._on_main_thread``).
"""

import wx

from .backend_agg import FigureCanvasAgg
from .backend_wx import _BackendWx, _FigureCanvasWxBase
from .backend_wx import (  # noqa: F401 # pylint: disable=W0611
    NavigationToolbar2Wx as NavigationToolbar2WxAgg)


class FigureCanvasWxAgg(FigureCanvasAgg, _FigureCanvasWxBase):
    def draw(self, drawDC=None):
        """
        Render the figure using agg.
        """
        FigureCanvasAgg.draw(self)
        self._on_main_thread(self._show_drawing, drawDC)

    def _show_drawing(self, drawDC):
        """Show what the canvas drew, on *drawDC* if given (`draw`'s end)."""
        self.bitmap = self._create_bitmap()
        self._isDrawn = True
        self.gui_repaint(drawDC=drawDC)

    def blit(self, bbox=None):
        # docstring inherited
        self._on_main_thread(self._blit, bbox)

    def _blit(self, bbox):
        """Show what the canvas holds inside *bbox* (`blit`'s work)."""
        bitmap = self._create_bitmap()
        if bbox is None:
            self.bitmap = bitmap
        else:
            srcDC = wx.MemoryDC(bitmap)
            destDC = wx.MemoryDC(self.bitmap)
            x = int(bbox.x0)
            y = int(self.bitmap.GetHeight() - bbox.y1)
            destDC.Blit(x, y, int(bbox.width), int(bbox.height), srcDC, x, y)
            destDC.SelectObject(wx.NullBitmap)
            srcDC.SelectObject(wx.NullBitmap)
        self.gui_repaint()

    def _create_bitmap(self):
        """Create a wx.Bitmap from the renderer RGBA buffer"""
        rgba = self.get_renderer().buffer_rgba()
        h, w, _ = rgba.shape
        bitmap = wx.Bitmap.FromBufferRGBA(w, h, rgba)
        bitmap.SetScaleFactor(self.GetDPIScaleFactor())
        return bitmap


@_BackendWx.export
class _BackendWxAgg(_BackendWx):
    FigureCanvas = FigureCanvasWxAgg
