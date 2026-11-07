"""
Compares what two programs saw: the records of their logs, event for
event, after leaving out what is not the program's to see (the times).
"""

import difflib
import json

#: Fields that differ between two runs of the same program.
_UNSTABLE = {"t"}


def normalise(records):
    """The records as compared: without the times and the ``ready``
    record (which names the window's layout)."""
    return [{key: value for key, value in record.items()
             if key not in _UNSTABLE}
            for record in records if record["name"] != "ready"]


#: The places Wayland does not tell: no client learns where the pointer
#: goes once it leaves its window, so pltrs reports leaving, and the keys
#: typed meanwhile, where the pointer was last (as Qt's Wayland platform
#: does, which also reports entering at the window's corner).
_PLACE = {"x", "y", "xdata", "ydata", "inaxes"}
#: The events whose places those are.
_UNPLACED = {"figure_enter_event", "figure_leave_event", "key_press_event",
             "key_release_event"}


def without_crossings(summarize):
    """*summarize* followed by leaving out the places Wayland does not
    tell, for comparing runs on Wayland, and the pointer leaving right
    before the window closes (the driver closes a Wayland window with the
    close button of its decoration)."""
    def summary(records):
        records = summarize(records)
        events = [record["name"] for record in records
                  if record["name"].endswith("_event")]
        if events[-2:] == ["figure_leave_event", "close_event"]:
            last_leave = max(i for i, record in enumerate(records)
                             if record["name"] == "figure_leave_event")
            records = records[:last_leave] + records[last_leave + 1:]
        return [{key: value for key, value in record.items()
                 if record["name"] not in _UNPLACED or key not in _PLACE}
                for record in records]
    return summary


def _summary(record):
    """A record in one line: its name and where, for aligning."""
    return json.dumps(record, sort_keys=True)


def _field_differences(index, left, right):
    """The lines naming the fields in which records *left* and *right*
    (the *index*-th of the reference) differ."""
    fields = sorted(set(left) | set(right))
    changed = [f"{field}: {left.get(field, '<none>')!r} != "
               f"{right.get(field, '<none>')!r}"
               for field in fields if left.get(field) != right.get(field)]
    if not changed:
        return []
    return [f"#{index} {left['name']}: " + "; ".join(changed)]


def compare(reference, records):
    """
    The lines describing how *records* differ from *reference*, empty
    when they are the same: records one has and the other lacks (matched
    by their names), and for the records both have the fields that
    differ.
    """
    left, right = normalise(reference), normalise(records)
    names = difflib.SequenceMatcher(
        None, [r["name"] for r in left], [r["name"] for r in right],
        autojunk=False)
    lines = []
    for tag, i1, i2, j1, j2 in names.get_opcodes():
        if tag == "equal":
            for offset in range(i2 - i1):
                lines += _field_differences(i1 + offset, left[i1 + offset],
                                            right[j1 + offset])
            continue
        for index in range(i1, i2):
            lines.append(f"#{index} only in the reference: "
                         f"{_summary(left[index])}")
        for index in range(j1, j2):
            lines.append(f"#{i1} only in pltrs: {_summary(right[index])}")
    return lines


#: The largest RMS difference two snapshots of the same figure may have,
#: as the spec's pixel comparisons allow.
SNAPSHOT_RMS = 1.0


def rms(left, right):
    """The root mean square difference of two equally long byte
    strings of pixels."""
    total = sum((a - b) ** 2 for a, b in zip(left, right))
    return (total / max(len(left), 1)) ** 0.5


def compare_snapshots(reference, snapshots, save_to=None):
    """
    The lines describing the snapshots of *snapshots* that differ from
    *reference*'s by more than `SNAPSHOT_RMS` (or in size); with
    *save_to*, a directory, both images of each such pair are written
    there as PNG files.
    """
    lines = []
    for name in sorted(set(reference) | set(snapshots)):
        if name not in reference or name not in snapshots:
            lines.append(f"snapshot {name}: only in "
                         f"{'pltrs' if name in snapshots else 'the reference'}")
            continue
        (lw, lh, left), (rw, rh, right) = reference[name], snapshots[name]
        if (lw, lh) != (rw, rh):
            lines.append(f"snapshot {name}: {lw}x{lh} != {rw}x{rh}")
            continue
        difference = rms(left, right)
        if difference > SNAPSHOT_RMS:
            lines.append(f"snapshot {name}: RMS {difference:.3f}")
            if save_to is not None:
                _save(save_to, name, (lw, lh), left, right)
    return lines


def _save(directory, name, size, left, right):
    """Write the two images of snapshot *name* as PNG files, if Pillow is
    there."""
    try:
        from PIL import Image
    except ImportError:
        return
    directory.mkdir(parents=True, exist_ok=True)
    for label, pixels in [("reference", left), ("pltrs", right)]:
        Image.frombytes("RGB", size, pixels).save(
            directory / f"{name}.{label}.png")
