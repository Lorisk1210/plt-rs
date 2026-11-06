"""
A figure embedded in a Qt application (``FigureCanvasQTAgg`` in a
``QMainWindow``, as matplotlib's ``embedding_in_qt`` example does), with
its navigation toolbar, a button that redraws, a blitting animation and a
resize. ``--matplotlib`` embeds real matplotlib instead of pltrs.
"""

from _embedding import TITLE, Embedding, arguments, package_module

args = arguments()
backend = package_module(args, "backends.backend_qtagg")
qt_compat = package_module(args, "backends.qt_compat")
QtCore, QtWidgets = qt_compat.QtCore, qt_compat.QtWidgets

app = QtWidgets.QApplication.instance() or QtWidgets.QApplication([])
window = QtWidgets.QMainWindow()
window.setWindowTitle(TITLE)
central = QtWidgets.QWidget()
window.setCentralWidget(central)
layout = QtWidgets.QVBoxLayout(central)

embedding = Embedding(args, lambda ms, function: QtCore.QTimer.singleShot(ms, function))
canvas = backend.FigureCanvasQTAgg(embedding.figure)
toolbar = backend.NavigationToolbar2QT(canvas, window)
button = QtWidgets.QPushButton("Redraw")
button.clicked.connect(embedding.redraw)
layout.addWidget(toolbar)
layout.addWidget(canvas)
layout.addWidget(button)

embedding.connect(
    canvas, [action.text() for action in toolbar.actions() if action.text()],
    widget_size=lambda: (canvas.width(), canvas.height()),
    click=button.click,
    resize=window.resize,
    close=window.close)
window.show()
embedding.start()
app.exec()
embedding.report()
