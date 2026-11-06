"""
A figure embedded in a Tk application (``FigureCanvasTkAgg`` in a Tk
window, as matplotlib's ``embedding_in_tk`` example does), with its
navigation toolbar, a button that redraws, a blitting animation and a
resize. ``--matplotlib`` embeds real matplotlib instead of pltrs.
"""

import tkinter as tk

from _embedding import TITLE, Embedding, arguments, package_module

args = arguments()
backend = package_module(args, "backends.backend_tkagg")

root = tk.Tk()
root.wm_title(TITLE)

embedding = Embedding(args, root.after)
canvas = backend.FigureCanvasTkAgg(embedding.figure, master=root)
toolbar = backend.NavigationToolbar2Tk(canvas, root, pack_toolbar=False)
toolbar.update()
button = tk.Button(master=root, text="Redraw", command=embedding.redraw)
button.pack(side=tk.BOTTOM)
toolbar.pack(side=tk.BOTTOM, fill=tk.X)
widget = canvas.get_tk_widget()
widget.pack(side=tk.TOP, fill=tk.BOTH, expand=True)


def toolbar_items():
    """The texts of the toolbar's buttons, in order."""
    return [child.cget("text") for child in toolbar.winfo_children()
            if isinstance(child, (tk.Button, tk.Checkbutton))]


embedding.connect(
    canvas, toolbar_items(),
    widget_size=lambda: (widget.winfo_width(), widget.winfo_height()),
    click=button.invoke,
    resize=lambda width, height: root.geometry(f"{width}x{height}"),
    close=root.destroy)
embedding.start()
root.mainloop()
embedding.report()
