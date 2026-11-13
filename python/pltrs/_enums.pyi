# The type stub of matplotlib._enums: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from enum import Enum


class JoinStyle(str, Enum):
    miter = "miter"
    round = "round"
    bevel = "bevel"
    @staticmethod
    def demo() -> None: ...


class CapStyle(str, Enum):
    butt = "butt"
    projecting = "projecting"
    round = "round"

    @staticmethod
    def demo() -> None: ...
