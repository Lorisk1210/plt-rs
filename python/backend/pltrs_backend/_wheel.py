"""
Writing wheels (the binary distribution format, PEP 427 and PEP 491):
a zip archive of the files to install, then its ``.dist-info``
directory with ``METADATA``, ``WHEEL``, the licence files and ``RECORD``,
which lists every file with its SHA-256 digest and size.

Archives are reproducible: entries are sorted and dated
``SOURCE_DATE_EPOCH`` (by default 1980-01-01, the earliest date zip
holds).
"""

import base64
import csv
import hashlib
import io
import os
import time
import zipfile

#: The earliest date a zip archive holds.
ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)


def archive_date():
    """The date of an archive's entries: ``SOURCE_DATE_EPOCH``, else
    1980-01-01."""
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if not epoch:
        return ZIP_EPOCH
    return max(ZIP_EPOCH, time.gmtime(int(epoch))[:6])


def record_digest(data):
    """``RECORD``'s digest of *data*: ``sha256=`` and the URL-safe base64
    of its SHA-256, unpadded."""
    digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
    return "sha256=" + digest.decode("ascii")


def dist_info_name(name, version):
    """The ``.dist-info`` directory of distribution *name* at *version*
    (the name normalised as wheel file names write it)."""
    return f"{normalise(name)}-{version}.dist-info"


def normalise(name):
    """A distribution name as wheel file names write it (PEP 427):
    runs of ``-``, ``_`` and ``.`` become ``_``, in lower case."""
    out, previous = [], False
    for char in name.lower():
        separator = char in "-_."
        if not (separator and previous):
            out.append("_" if separator else char)
        previous = separator
    return "".join(out)


def wheel_file(tags, pure):
    """The text of a wheel's ``WHEEL`` file, for its *tags* (``cp312-abi3-
    win_amd64``, ...), its files installed in ``purelib`` when *pure*."""
    lines = [
        "Wheel-Version: 1.0",
        "Generator: pltrs_backend",
        f"Root-Is-Purelib: {'true' if pure else 'false'}",
        *(f"Tag: {tag}" for tag in tags),
    ]
    return "\n".join(lines) + "\n"


class WheelWriter:
    """Collects a wheel's files, then writes the archive."""

    def __init__(self, name, version):
        self.name = name
        self.version = version
        self.dist_info = dist_info_name(name, version)
        self._files = {}

    def add(self, archive_path, data, executable=False):
        """Adds *data* (bytes) as *archive_path* (``/``-separated)."""
        self._files[archive_path] = (data, executable)

    def add_dist_info(self, relative, data):
        """Adds *data* as *relative* in the ``.dist-info`` directory."""
        self.add(f"{self.dist_info}/{relative}", data)

    def add_data(self, relative, data):
        """Adds *data* as *relative* in the ``.data/data`` directory,
        which installers put under the installation's prefix
        (``share/...`` lands in ``~/.local/share`` with ``--user``)."""
        data_dir = self.dist_info.removesuffix(".dist-info") + ".data"
        self.add(f"{data_dir}/data/{relative}", data)

    def write(self, directory, tags, pure):
        """
        Writes the wheel for *tags* into *directory*, with its ``WHEEL``
        and ``RECORD``, and returns its file name (the tags compressed as
        PEP 425 does: ``cp312-abi3-a.b`` for two platforms).
        """
        self.add_dist_info("WHEEL", wheel_file(tags, pure).encode())
        interpreters = ".".join(dict.fromkeys(tag.split("-")[0] for tag in tags))
        abis = ".".join(dict.fromkeys(tag.split("-")[1] for tag in tags))
        platforms = ".".join(dict.fromkeys(tag.split("-")[2] for tag in tags))
        filename = (f"{normalise(self.name)}-{self.version}-"
                    f"{interpreters}-{abis}-{platforms}.whl")
        record_path = f"{self.dist_info}/RECORD"
        record = io.StringIO()
        rows = csv.writer(record, lineterminator="\n")
        for path, (data, _) in sorted(self._files.items()):
            rows.writerow([path, record_digest(data), len(data)])
        rows.writerow([record_path, "", ""])
        entries = sorted(self._files.items(), key=lambda item: (
            item[0].startswith(self.dist_info), item[0]))
        entries.append((record_path, (record.getvalue().encode(), False)))
        date = archive_date()
        with zipfile.ZipFile(os.path.join(directory, filename), "w",
                             compression=zipfile.ZIP_DEFLATED) as archive:
            for path, (data, executable) in entries:
                info = zipfile.ZipInfo(path, date_time=date)
                info.external_attr = (0o755 if executable else 0o644) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, data)
        return filename
