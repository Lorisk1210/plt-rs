"""
3-D plotting (matplotlib's ``mpl_toolkits.mplot3d``): `.Axes3D`, made by
``fig.add_subplot(projection='3d')``, over pltrs (Rust)'s 3-D axes.
"""

from .axes3d import Axes3D

__all__ = ['Axes3D']
