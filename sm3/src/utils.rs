use block_buffer::byteorder::{ByteOrder, BE};

use crate::consts::{T_0, T_1};

#[inline(always)]
fn ff0(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

#[inline(always)]
fn ff1(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (x & z) | (y & z)
}

#[inline(always)]
fn gg0(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}

#[inline(always)]
fn gg1(x: u32, y: u32, z: u32) -> u32 {
    (x & y) | (!x & z)
}

#[inline(always)]
fn p0(x: u32) -> u32 {
    x ^ x.rotate_left(9) ^ x.rotate_left(17)
}

#[inline(always)]
fn p1(x: u32) -> u32 {
    x ^ x.rotate_left(15) ^ x.rotate_left(23)
}

fn sm3_digest_w(state: &mut [u32; 8], w: &[u32; 68], w_prime: &[u32; 64]) {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for j in 0..=15 {
        let ss1 = a
            .rotate_left(12)
            .wrapping_add(e)
            .wrapping_add(T_0.rotate_left(j as u32))
            .rotate_left(7);
        let ss2 = ss1 ^ a.rotate_left(12);
        let tt1 = ff0(a, b, c)
            .wrapping_add(d)
            .wrapping_add(ss2)
            .wrapping_add(w_prime[j]);
        let tt2 = gg0(e, f, g)
            .wrapping_add(h)
            .wrapping_add(ss1)
            .wrapping_add(w[j]);
        d = c;
        c = b.rotate_left(9);
        b = a;
        a = tt1;
        h = g;
        g = f.rotate_left(19);
        f = e;
        e = p0(tt2);
    }
    for j in 16..=63 {
        let ss1 = a
            .rotate_left(12)
            .wrapping_add(e)
            .wrapping_add(T_1.rotate_left(j as u32))
            .rotate_left(7);
        let ss2 = ss1 ^ a.rotate_left(12);
        let tt1 = ff1(a, b, c)
            .wrapping_add(d)
            .wrapping_add(ss2)
            .wrapping_add(w_prime[j]);
        let tt2 = gg1(e, f, g)
            .wrapping_add(h)
            .wrapping_add(ss1)
            .wrapping_add(w[j]);
        d = c;
        c = b.rotate_left(9);
        b = a;
        a = tt1;
        h = g;
        g = f.rotate_left(19);
        f = e;
        e = p0(tt2);
    }
    *state = [
        state[0] ^ a,
        state[1] ^ b,
        state[2] ^ c,
        state[3] ^ d,
        state[4] ^ e,
        state[5] ^ f,
        state[6] ^ g,
        state[7] ^ h,
    ];
}

/// CF: compress function
pub fn compress256(state: &mut [u32; 8], block: &[u8; 64]) {
    // 5.3.2 Message Expansion
    let mut w = [0u32; 68];
    let mut w_prime = [0u32; 64];

    BE::read_u32_into(block, &mut w[..16]);
    for j in 16..=67 {
        w[j] = p1(w[j - 16] ^ w[j - 9] ^ w[j - 3].rotate_left(15))
            ^ w[j - 13].rotate_left(7)
            ^ w[j - 6];
    }
    for j in 0..=63 {
        w_prime[j] = w[j] ^ w[j + 4];
    }

    sm3_digest_w(state, &w, &w_prime);
}
