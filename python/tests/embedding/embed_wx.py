"""
A figure embedded in a wxPython application (``FigureCanvasWxAgg`` in a
``wx.Frame``, as matplotlib's ``embedding_in_wx2`` example does), with its
navigation toolbar, a button that redraws, a blitting animation and a
resize. ``--matplotlib`` embeds real matplotlib instead of pltrs.
"""

import wx

from _embedding import TITLE, Embedding, arguments, package_module

args = arguments()
backend = package_module(args, "backends.backend_wxagg")
NavigationToolbar2Wx = package_module(args, "backends.backend_wx").NavigationToolbar2Wx

app = wx.App()
frame = wx.Frame(None, title=TITLE)
embedding = Embedding(args, wx.CallLater)
canvas = backend.FigureCanvasWxAgg(frame, -1, embedding.figure)
toolbar = NavigationToolbar2Wx(canvas)
toolbar.Realize()
button = wx.Button(frame, label="Redraw")
button.Bind(wx.EVT_BUTTON, lambda event: embedding.redraw())
sizer = wx.BoxSizer(wx.VERTICAL)
sizer.Add(toolbar, 0, wx.EXPAND)
sizer.Add(canvas, 1, wx.EXPAND)
sizer.Add(button, 0)
frame.SetSizer(sizer)
frame.Fit()


def click():
    """Press the Redraw button as a click would."""
    event = wx.CommandEvent(wx.wxEVT_BUTTON, button.GetId())
    event.SetEventObject(button)
    button.GetEventHandler().ProcessEvent(event)


embedding.connect(
    canvas, [toolbar.GetToolByPos(i).GetLabel() for i in range(toolbar.GetToolsCount())
             if toolbar.GetToolByPos(i).GetLabel()],
    widget_size=lambda: tuple(canvas.GetSize()),
    click=click,
    resize=lambda width, height: frame.SetSize(width, height),
    close=frame.Close)
frame.Show()
embedding.start()
app.MainLoop()
embedding.report()
