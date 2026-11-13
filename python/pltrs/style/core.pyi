# The type stub of matplotlib.style.core: matplotlib 3.11.2's, under the
# Matplotlib License, importing pltrs' modules for matplotlib's
# (python/tools/matplotlib_stubs.py).

from collections.abc import Generator
import contextlib

from pltrs import RcParams
from pltrs.typing import RcStyleType

USER_LIBRARY_PATHS: list[str] = ...
BASE_LIBRARY_PATH: str = ...
STYLE_EXTENSION: str = ...
STYLE_BLACKLIST: set[str] = ...

def use(style: RcStyleType) -> None: ...
@contextlib.contextmanager
def context(
    style: RcStyleType, after_reset: bool = ...
) -> Generator[None, None, None]: ...

library: dict[str, RcParams]
available: list[str]

def reload_library() -> None: ...

__all__ = [
    "use", "context", "available", "library", "reload_library",
    "USER_LIBRARY_PATHS", "BASE_LIBRARY_PATH", "STYLE_EXTENSION", "STYLE_BLACKLIST",
]
