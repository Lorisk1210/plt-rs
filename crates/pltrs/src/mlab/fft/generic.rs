//! pocketfft's pass for a radix it has no hard-coded butterfly for
//! (`cfftp::passg`, the primes from 13 on), with its operations in its
//! order. It works in place: the result replaces the input, the output
//! buffer serving as scratch.

use super::super::Complex;
use super::radix::{Layout, pm, special_mul};

/// pocketfft's `passg`: `cc` holds the input and receives the result;
/// `roots` are the `radix`-th roots of unity (`csarr`).
pub(super) fn pass_generic<const FWD: bool>(
    p: Layout<'_>,
    roots: &[Complex],
    cc: &mut [Complex],
    ch: &mut [Complex],
) {
    let (ido, l1, ip) = (p.ido, p.l1, p.radix);
    let half = ip.div_ceil(2);
    let idl1 = ido * l1;
    let wal: Vec<Complex> = roots
        .iter()
        .enumerate()
        .map(|(i, root)| match i {
            0 => Complex::ONE,
            _ if FWD => root.conj(),
            _ => *root,
        })
        .collect();
    // `CX(a, b, c)` addresses `cc` with the output's layout.
    let cx = |a: usize, b: usize, c: usize| a + ido * (b + l1 * c);

    for k in 0..l1 {
        for i in 0..ido {
            ch[p.output(i, k, 0)] = cc[p.input(i, 0, k)];
        }
    }
    for (j, jc) in (1..half).zip((half..ip).rev()) {
        for k in 0..l1 {
            for i in 0..ido {
                let (sum, difference) = pm(cc[p.input(i, j, k)], cc[p.input(i, jc, k)]);
                ch[p.output(i, k, j)] = sum;
                ch[p.output(i, k, jc)] = difference;
            }
        }
    }
    for k in 0..l1 {
        for i in 0..ido {
            let total = (1..half).fold(ch[p.output(i, k, 0)], |acc, j| acc + ch[p.output(i, k, j)]);
            cc[cx(i, k, 0)] = total;
        }
    }
    for (l, lc) in (1..half).zip((half..ip).rev()) {
        combine::<FWD>(ip, l, lc, idl1, &wal, cc, ch);
    }
    shuffle_and_twiddle::<FWD>(p, half, cc);
}

/// The outputs `l` and `lc = ip - l` of every point, from the sums and
/// differences in `ch` (the loop over `l` of `passg`).
fn combine<const FWD: bool>(
    ip: usize,
    l: usize,
    lc: usize,
    idl1: usize,
    wal: &[Complex],
    cc: &mut [Complex],
    ch: &[Complex],
) {
    let half = ip.div_ceil(2);
    let cx2 = |a: usize, b: usize| a + idl1 * b;
    let ch2 = |a: usize, b: usize| ch[a + idl1 * b];
    let (w1, w2) = (wal[l], wal[2 * l]);
    for ik in 0..idl1 {
        let (c0, c1, c2) = (ch2(ik, 0), ch2(ik, 1), ch2(ik, 2));
        let (d1, d2) = (ch2(ik, ip - 1), ch2(ik, ip - 2));
        cc[cx2(ik, l)] = Complex::new(
            c0.re + w1.re * c1.re + w2.re * c2.re,
            c0.im + w1.re * c1.im + w2.re * c2.im,
        );
        cc[cx2(ik, lc)] = Complex::new(
            -w1.im * d1.im - w2.im * d2.im,
            w1.im * d1.re + w2.im * d2.re,
        );
    }
    let mut iwal = 2 * l;
    let mut step = || {
        iwal += l;
        if iwal > ip {
            iwal -= ip;
        }
        wal[iwal]
    };
    let (mut j, mut jc) = (3, ip - 3);
    while j + 1 < half {
        let (xwal, xwal2) = (step(), step());
        for ik in 0..idl1 {
            let (a, b) = (ch2(ik, j), ch2(ik, j + 1));
            let (c, d) = (ch2(ik, jc), ch2(ik, jc - 1));
            let out = &mut cc[cx2(ik, l)];
            *out = Complex::new(
                out.re + (a.re * xwal.re + b.re * xwal2.re),
                out.im + (a.im * xwal.re + b.im * xwal2.re),
            );
            let out = &mut cc[cx2(ik, lc)];
            *out = Complex::new(
                out.re - (c.im * xwal.im + d.im * xwal2.im),
                out.im + (c.re * xwal.im + d.re * xwal2.im),
            );
        }
        j += 2;
        jc -= 2;
    }
    while j < half {
        let xwal = step();
        for ik in 0..idl1 {
            let (a, c) = (ch2(ik, j), ch2(ik, jc));
            let out = &mut cc[cx2(ik, l)];
            *out = Complex::new(out.re + a.re * xwal.re, out.im + a.im * xwal.re);
            let out = &mut cc[cx2(ik, lc)];
            *out = Complex::new(out.re - c.im * xwal.im, out.im + c.re * xwal.im);
        }
        j += 1;
        jc -= 1;
    }
}

/// The last step of `passg`: each output pair `j`, `ip - j` becomes its
/// sum and difference, rotated by the twiddle factors past the first
/// point.
fn shuffle_and_twiddle<const FWD: bool>(p: Layout<'_>, half: usize, cc: &mut [Complex]) {
    let (ido, l1, ip) = (p.ido, p.l1, p.radix);
    let cx = |a: usize, b: usize, c: usize| a + ido * (b + l1 * c);
    for (j, jc) in (1..half).zip((half..ip).rev()) {
        for k in 0..l1 {
            for i in 0..ido {
                let (x1, x2) = pm(cc[cx(i, k, j)], cc[cx(i, k, jc)]);
                if i == 0 {
                    cc[cx(i, k, j)] = x1;
                    cc[cx(i, k, jc)] = x2;
                } else {
                    cc[cx(i, k, j)] = special_mul::<FWD>(x1, p.twiddle(j - 1, i));
                    cc[cx(i, k, jc)] = special_mul::<FWD>(x2, p.twiddle(jc - 1, i));
                }
            }
        }
    }
}
