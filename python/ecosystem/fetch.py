"""
Download what ``cargo xtask ecosystem suite`` runs the libraries' test
suites from, checking every download's SHA-256.

``cargo xtask ecosystem setup`` runs it as

* ``fetch.py sdist NAME VERSION DEST``: looks the release up in PyPI's
  JSON API, downloads its sdist, checks the SHA-256 PyPI gives for it and
  unpacks it into the directory ``DEST``, with the archive filter that
  refuses links and paths leading out of the directory. The archive is
  unpacked beside, then its one top directory (``NAME-VERSION``, or the
  normalised ``scikit_learn-1.9.1``) is renamed into place, so that
  ``DEST`` exists only complete;
* ``fetch.py archive URL SHA256 DEST``: downloads the source archive of a
  release's tag (the tests of the libraries whose sdists ship none), checks
  its SHA-256 and unpacks it into ``DEST`` as an sdist is;
* ``fetch.py file URL SHA256 DEST``: downloads one file (a test's data
  that the sdist leaves out) to ``DEST``, if its SHA-256 is *SHA256*;
* ``fetch.py zip URL SHA256 DEST``: downloads a checked zip archive (a
  dataset yellowbrick reads unpacked), unpacks it into the directory of
  ``DEST``, refusing entries that lead out of it, then writes the archive
  to ``DEST``, so that ``DEST`` exists only once its entries do.

It uses the standard library alone.
"""

import hashlib
import http.client
import io
import json
import pathlib
import shutil
import sys
import tarfile
import urllib.request
import zipfile

#: PyPI's JSON description of a release.
RELEASE_URL = "https://pypi.org/pypi/{name}/{version}/json"

#: The user agent the downloads name: some hosts (scverse's example data)
#: refuse urllib's own.
USER_AGENT = "pltrs-ecosystem-fetch"


#: How long a download may stall, in seconds, and how often it is tried:
#: a connection that stalls or breaks off midway (seen on the Windows PC's
#: line, 2026-11-18) is tried again rather than waited on for ever.
TIMEOUT = 60
ATTEMPTS = 4


def download(url, sha256):
    """The bytes at *url*, if their SHA-256 is *sha256*."""
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(1, ATTEMPTS + 1):
        try:
            with urllib.request.urlopen(request, timeout=TIMEOUT) as answer:
                data = answer.read()
            break
        except (OSError, http.client.HTTPException) as error:
            if attempt == ATTEMPTS:
                raise
            print(f"{url}: {error}; trying again", flush=True)
    digest = hashlib.sha256(data).hexdigest()
    if digest != sha256:
        raise SystemExit(f"{url}: SHA-256 {digest}, expected {sha256}")
    return data


def sdist_of(name, version):
    """The URL and SHA-256 of the sdist of *name* *version* on PyPI."""
    with urllib.request.urlopen(RELEASE_URL.format(name=name, version=version),
                                timeout=TIMEOUT) as answer:
        release = json.load(answer)
    for file in release["urls"]:
        if file["packagetype"] == "sdist":
            return file["url"], file["digests"]["sha256"]
    raise SystemExit(f"{name} {version} has no sdist on PyPI")


def sdist(name, version, dest):
    """Downloads, checks and unpacks the sdist into *dest*."""
    archive(*sdist_of(name, version), dest)


def archive(url, sha256, dest):
    """Downloads, checks and unpacks the archive at *url* into *dest*."""
    data = download(url, sha256)
    # Beside it under a short name: Windows' paths hold 259 characters at
    # most, which scanpy's deepest test images come near.
    partial = dest.with_name(".partial")
    shutil.rmtree(partial, ignore_errors=True)
    partial.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        tar.extractall(partial, filter="data")
    match list(partial.iterdir()):
        case [top] if top.is_dir():
            top.rename(dest)
        case entries:
            raise SystemExit(f"{url}: {len(entries)} entries at the top, expected one directory")
    shutil.rmtree(partial)
    print(f"unpacked {url} into {dest}")


def file(url, sha256, dest):
    """Downloads one checked file to *dest*."""
    data = download(url, sha256)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)
    print(f"downloaded {url} to {dest}")


def unzip(url, sha256, dest):
    """Downloads the checked zip archive at *url*, unpacks it beside *dest*
    and writes it to *dest*."""
    data = download(url, sha256)
    home = dest.parent.resolve()
    home.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for name in archive.namelist():
            if not (home / name).resolve().is_relative_to(home):
                raise SystemExit(f"{url}: {name} leads out of {home}")
        archive.extractall(home)
    dest.write_bytes(data)
    print(f"unpacked {url} into {home}")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["sdist", name, version, dest]:
            sdist(name, version, pathlib.Path(dest))
        case ["archive", url, sha256, dest]:
            archive(url, sha256, pathlib.Path(dest))
        case ["file", url, sha256, dest]:
            file(url, sha256, pathlib.Path(dest))
        case ["zip", url, sha256, dest]:
            unzip(url, sha256, pathlib.Path(dest))
        case _:
            raise SystemExit(
                "usage: fetch.py sdist NAME VERSION DEST | archive URL SHA256 DEST"
                " | file URL SHA256 DEST | zip URL SHA256 DEST"
            )
