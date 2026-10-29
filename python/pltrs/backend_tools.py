"""
Abstract base classes define the primitives for Tools (matplotlib's
``backend_tools``).

pltrs has the cursor types the navigation toolbar sets.
"""

import enum


class Cursors(enum.IntEnum):  # Must subclass int for the macOS backend.
    """Backend-independent cursor types."""
    POINTER = enum.auto()
    HAND = enum.auto()
    SELECT_REGION = enum.auto()
    MOVE = enum.auto()
    WAIT = enum.auto()
    RESIZE_HORIZONTAL = enum.auto()
    RESIZE_VERTICAL = enum.auto()


cursors = Cursors  # Backcompat.
