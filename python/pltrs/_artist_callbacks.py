"""
The property-change callbacks of every artist (matplotlib's
``Artist.add_callback``, ``remove_callback`` and ``pchanged``).
"""

from . import cbook


class CallbacksMixin:
    """The callback methods of `.Artist`."""

    def _property_callbacks(self):
        """The artist's registry of ``pchanged`` callbacks, made when first
        needed."""
        if "_callbacks" not in self.__dict__:
            self._callbacks = cbook.CallbackRegistry(signals=["pchanged"])
        return self._callbacks

    def add_callback(self, func):
        """
        Add a callback function that will be called whenever one of the
        `.Artist`'s properties changes.

        Parameters
        ----------
        func : callable
            The callback function. It must have the signature::

                def func(artist: Artist) -> Any

            where *artist* is the calling `.Artist`. Return values may exist
            but are ignored.

        Returns
        -------
        int
            The observer id associated with the callback. This id can be
            used for removing the callback with `.remove_callback` later.

        See Also
        --------
        remove_callback
        """
        # Wrapping func in a lambda ensures it can be connected multiple times
        # and never gets weakref-gc'ed.
        return self._property_callbacks().connect("pchanged",
                                                  lambda: func(self))

    def remove_callback(self, oid):
        """
        Remove a callback based on its observer id.

        See Also
        --------
        add_callback
        """
        self._property_callbacks().disconnect(oid)

    def pchanged(self):
        """
        Call all of the registered callbacks.

        This function is triggered internally when a property is changed.

        See Also
        --------
        add_callback
        remove_callback
        """
        if "_callbacks" in self.__dict__:
            self._callbacks.process("pchanged")
