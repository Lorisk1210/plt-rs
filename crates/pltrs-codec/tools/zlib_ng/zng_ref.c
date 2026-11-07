/* Reference compressor for checking pltrs's port of zlib-ng's deflate.
 *
 *   zng_ref oneshot LEVEL MEMLEVEL STRATEGY WBITS < in > out
 *   zng_ref pillow  LEVEL MEMLEVEL STRATEGY WBITS ROWLEN BUFSIZE < in > out
 *
 * oneshot: all input and a deflateBound output buffer, deflate(Z_FINISH)
 * until the end (what compress2 does).
 * pillow: Pillow's ImagingZipEncode loop: rows of ROWLEN bytes fed one at a
 * time with Z_NO_FLUSH into output buffers of BUFSIZE bytes, then Z_FINISH.
 * The chunk sizes (one per encode call) go to stderr, one per line.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "zlib.h"

static unsigned char *read_all(size_t *len) {
    size_t cap = 1 << 20, n = 0;
    unsigned char *buf = malloc(cap);
    for (;;) {
        if (n == cap) {
            cap *= 2;
            buf = realloc(buf, cap);
        }
        size_t got = fread(buf + n, 1, cap - n, stdin);
        if (got == 0)
            break;
        n += got;
    }
    *len = n;
    return buf;
}

static int init(z_stream *strm, char **argv) {
    memset(strm, 0, sizeof(*strm));
    return deflateInit2(strm, atoi(argv[2]), Z_DEFLATED, atoi(argv[5]), atoi(argv[3]), atoi(argv[4]));
}

static int oneshot(char **argv) {
    size_t len;
    unsigned char *in = read_all(&len);
    z_stream strm;
    if (init(&strm, argv) != Z_OK)
        return 2;
    size_t cap = deflateBound(&strm, len) + 64;
    unsigned char *out = malloc(cap);
    strm.next_in = in;
    strm.avail_in = (uInt)len;
    strm.next_out = out;
    strm.avail_out = (uInt)cap;
    int err;
    do {
        err = deflate(&strm, Z_FINISH);
    } while (err == Z_OK);
    if (err != Z_STREAM_END)
        return 3;
    fwrite(out, 1, strm.total_out, stdout);
    deflateEnd(&strm);
    return 0;
}

/* One call of Pillow's ImagingZipEncode with `bytes` of output room. */
static int pillow_encode(z_stream *strm, int *state, const unsigned char *in, size_t rows, size_t row_len,
                         size_t *y, unsigned char *buf, unsigned bytes, int *done) {
    strm->next_out = buf;
    strm->avail_out = bytes;
    if (strm->next_in && strm->avail_in > 0) {
        if (deflate(strm, Z_NO_FLUSH) < 0)
            return -1;
    }
    for (;;) {
        switch (*state) {
        case 1:
            while (strm->avail_out > 0) {
                if (*y >= rows) {
                    *state = 2;
                    break;
                }
                strm->next_in = (unsigned char *)in + *y * row_len;
                strm->avail_in = (uInt)row_len;
                (*y)++;
                if (deflate(strm, Z_NO_FLUSH) < 0)
                    return -1;
            }
            if (strm->avail_out == 0)
                break;
            /* fall through */
        case 2:
            while (strm->avail_out > 0) {
                int err = deflate(strm, Z_FINISH);
                if (err == Z_STREAM_END) {
                    *done = 1;
                    break;
                }
                if (strm->avail_out == 0)
                    break;
            }
        }
        return (int)(bytes - strm->avail_out);
    }
}

static int pillow(char **argv) {
    size_t len;
    unsigned char *in = read_all(&len);
    size_t row_len = (size_t)atol(argv[6]);
    unsigned bufsize = (unsigned)atol(argv[7]);
    if (row_len == 0 || len % row_len != 0)
        return 4;
    z_stream strm;
    if (init(&strm, argv) != Z_OK)
        return 2;
    unsigned char *buf = malloc(bufsize);
    int state = 1, done = 0;
    size_t y = 0;
    while (!done) {
        int n = pillow_encode(&strm, &state, in, len / row_len, row_len, &y, buf, bufsize, &done);
        if (n < 0)
            return 3;
        fwrite(buf, 1, (size_t)n, stdout);
        fprintf(stderr, "%d\n", n);
    }
    deflateEnd(&strm);
    return 0;
}

int main(int argc, char **argv) {
    if (argc >= 6 && strcmp(argv[1], "oneshot") == 0)
        return oneshot(argv);
    if (argc >= 8 && strcmp(argv[1], "pillow") == 0)
        return pillow(argv);
    fprintf(stderr, "usage: zng_ref oneshot|pillow LEVEL MEMLEVEL STRATEGY WBITS [ROWLEN BUFSIZE]\n");
    return 1;
}
