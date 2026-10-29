"""
matplotlib's toolkits for pltrs: ``pltrs.mpl_toolkits.mplot3d``,
``axes_grid1`` and ``axisartist``.

Code here imports pltrs by name (``from pltrs import colors``), as
matplotlib's toolkits import matplotlib: running matplotlib's suite loads
this package as the top-level ``mpl_toolkits``, where a relative import of
pltrs would not reach it.
"""
