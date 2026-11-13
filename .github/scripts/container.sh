#!/bin/sh
# Tests pltrs (Python)'s wheels inside a manylinux or musllinux container
# (quay.io/pypa/...), with the source tree as the working directory:
#
#   .github/scripts/container.sh test DIST...   DIST... installed and checked with the container's
#                                               oldest Python (python/tools/check_dist.py --install)
#
# The container has the oldest C library the wheel's tag allows (glibc
# 2.17 in manylinux2014, musl 1.2 in musllinux_1_2): the test runs the
# wheel there, with the numpy and Pillow pip picks for it.
set -eu

python=/opt/python/cp312-cp312/bin/python

case "${1:-}" in
    test)
        shift
        "$python" python/tools/check_dist.py --install --python "$python" \
            --work /tmp/check-dist "$@"
        ;;
    *)
        echo "usage: $0 test DIST..." >&2
        exit 2
        ;;
esac
