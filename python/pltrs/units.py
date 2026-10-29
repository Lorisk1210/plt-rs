"""
The unit conversion interface (matplotlib's ``units``): the registry of
converters for types of plotting values.
"""


class Registry(dict):
    """Register types with conversion interface."""


#: The converters of every registered type.
registry = Registry()
