"""
matplotlib's Sphinx extensions (matplotlib's package, whose ``__init__`` is
empty too): `.plot_directive`, `.mathmpl`, `.figmpl_directive` and
`.roles`, which need Sphinx and docutils.

The four modules are matplotlib 3.11.2's, under the Matplotlib License,
kept as they are but for their imports of the package, which are relative
or name the package by the name it was loaded under (``matplotlib`` or
``pltrs``; also in the default *plot_pre_code* of the plot directive).
Each keeps matplotlib's line numbers, which the tracebacks of a failing
plot in a documentation build show.
"""
