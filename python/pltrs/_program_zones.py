"""
Time zones of the program's: a `datetime.tzinfo` pltrs (Rust) has no model
of (``dateutil.tz.tzlocal()``, ``dateutil.tz.tzstr``, a class the program
writes) is given to pltrs (Rust) as a `ProgramZone`, which pltrs (Rust)
asks what it needs (``pltrs::dates::ZoneRules``). matplotlib only calls a
``tzinfo``'s methods, through `datetime` objects in it; a `ProgramZone`
asks the zone the same way: ``astimezone`` to and from UTC, as matplotlib's
`.date2num` and `.num2date` convert, and ``tzname``, as ``strftime`` asks.

The names pltrs (Rust) finds no zone of (a POSIX ``TZ`` string, an empty
name for the system's local zone) are read with dateutil's ``gettz``, as
matplotlib reads the :rc:`timezone` rcParam (`find`).
"""

import datetime

from ._native import register as _register

_UTC = datetime.timezone.utc
_MICROSECOND = datetime.timedelta(microseconds=1)


def _parts(dt):
    """The calendar parts of *dt*, as pltrs (Rust) takes them."""
    return ((dt.year, dt.month, dt.day),
            (dt.hour, dt.minute, dt.second, dt.microsecond))


class ProgramZone:
    """What pltrs (Rust) calls to ask the zone *tz* (kept as ``tz``)."""

    def __init__(self, tz):
        self.tz = tz

    def offsets(self, times):
        """The offsets from UTC, in microseconds, of the wall-clock times
        *times* (``[(parts, fold)]``): as matplotlib's `.date2num` finds
        each one's instant (``dt.astimezone(UTC)``)."""
        tz = self.tz
        offsets = []
        for (date, time), fold in times:
            local = datetime.datetime(*date, *time, tzinfo=tz, fold=fold)
            utc = local.astimezone(_UTC)
            offsets.append(
                (local.replace(tzinfo=None) - utc.replace(tzinfo=None))
                // _MICROSECOND)
        return offsets

    def name(self, parts, fold):
        """The zone's name at the wall-clock time of *parts* (``tzname``),
        '' when it gives none."""
        date, time = parts
        name = datetime.datetime(*date, *time, tzinfo=self.tz,
                                 fold=fold).tzname()
        return '' if name is None else str(name)

    def local(self, parts):
        """The wall-clock parts and fold of the UTC instant of *parts*, as
        matplotlib's `.num2date` finds them (``astimezone(tz)``)."""
        date, time = parts
        local = datetime.datetime(*date, *time, tzinfo=_UTC).astimezone(self.tz)
        return _parts(local), bool(local.fold)


def find(name):
    """The zone dateutil's ``gettz`` finds for *name*, where pltrs (Rust)
    finds none, as pltrs (Rust) takes it; None for none."""
    import dateutil.tz
    tz = dateutil.tz.gettz(name)
    return None if tz is None else ProgramZone(tz)


_register("find_zone", find)
