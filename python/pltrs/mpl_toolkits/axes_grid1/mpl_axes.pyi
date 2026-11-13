# A module of pltrs' toolkits: untyped for type checkers, as matplotlib's
# mpl_toolkits are (python/tools/matplotlib_stubs.py).

from typing import Any

def __getattr__(name: str) -> Any: ...
