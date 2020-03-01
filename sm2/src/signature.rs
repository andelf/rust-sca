// Copyright (C) 2018
//
// This file is part of libsm.
//
// libsm is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// libsm is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with libsm.  If not, see <http://www.gnu.org/licenses/>.

use std::fmt;
use byteorder::{BigEndian, WriteBytesExt};
use digest::Digest;
use num_bigint::BigUint;
use num_traits::*;
use sm3::Sm3;

use super::ecc::{EccCtx, Point};
use super::field::FieldElem;

pub type Pubkey = super::ecc::Point;
pub type Seckey = BigUint;

pub struct Signature {
    r: BigUint,
    s: BigUint,
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Signature")
            .field("r", &format!("{:x}", self.r))
            .field("s", &format!("{:x}", self.s))
            .finish()
    }
}

impl Signature {
    pub fn der_decode(buf: &[u8]) -> Result<Signature, yasna::ASN1Error> {
        let (r, s) = yasna::parse_der(buf, |reader| {
            reader.read_sequence(|reader| {
                let r = reader.next().read_biguint()?;
                let s = reader.next().read_biguint()?;
                return Ok((r, s));
            })
        })?;
        Ok(Signature { r, s })
    }

    pub fn der_decode_raw(buf: &[u8]) -> Result<Signature, bool> {
        if buf[0] != 0x02 {
            return Err(true);
        }
        let r_len: usize = buf[1] as usize;
        if buf.len() <= r_len + 4 {
            return Err(true);
        }
        let r = BigUint::from_bytes_be(&buf[2..2 + r_len]);

        let buf = &buf[2 + r_len..];
        if buf[0] != 0x02 {
            return Err(true);
        }
        let s_len: usize = buf[1] as usize;
        if buf.len() < s_len + 2 {
            return Err(true);
        }
        let s = BigUint::from_bytes_be(&buf[2..2 + s_len]);

        return Ok(Signature { r, s });
    }

    pub fn der_encode(&self) -> Vec<u8> {
        let der = yasna::construct_der(|writer| {
            writer.write_sequence(|writer| {
                writer.next().write_biguint(&self.r);
                writer.next().write_biguint(&self.s);
            })
        });
        return der;
    }
}

pub struct SigCtx {
    curve: EccCtx,
}

impl SigCtx {
    pub fn new() -> SigCtx {
        SigCtx {
            curve: EccCtx::new(),
        }
    }

    pub fn hash(&self, id: &str, pk: &Point, msg: &[u8]) -> [u8; 32] {
        let curve = &self.curve;

        let mut prepend: Vec<u8> = Vec::new();
        if id.len() * 8 > 65535 {
            panic!("ID is too long.");
        }
        prepend
            .write_u16::<BigEndian>((id.len() * 8) as u16)
            .unwrap();
        for c in id.bytes() {
            prepend.push(c);
        }

        let mut a = curve.get_a();
        let mut b = curve.get_b();

        prepend.append(&mut a);
        prepend.append(&mut b);

        let (x_g, y_g) = curve.to_affine(&curve.generator());
        let (mut x_g, mut y_g) = (x_g.to_bytes(), y_g.to_bytes());
        prepend.append(&mut x_g);
        prepend.append(&mut y_g);

        let (x_a, y_a) = curve.to_affine(pk);
        let (mut x_a, mut y_a) = (x_a.to_bytes(), y_a.to_bytes());
        prepend.append(&mut x_a);
        prepend.append(&mut y_a);

        let mut hasher = Sm3::new();
        hasher.input(&prepend[..]);
        let z_a = hasher.result();

        // Z_A = HASH_256(ID_LEN || ID || x_G || y_G || x_A || y_A)

        // e = HASH_256(Z_A || M)

        let mut prepended_msg: Vec<u8> = Vec::new();
        prepended_msg.extend_from_slice(&z_a[..]);
        prepended_msg.extend_from_slice(&msg[..]);

        let mut hasher = Sm3::new();
        hasher.input(&prepended_msg[..]);
        hasher.result().into()
    }

    pub fn sign(&self, msg: &[u8], sk: &BigUint, pk: &Point) -> Signature {
        // Get the value "e", which is the hash of message and ID, EC parameters and public key
        let digest = self.hash("1234567812345678", pk, msg);

        self.sign_raw(&digest[..], sk)
    }

    pub fn sign_raw(&self, digest: &[u8], sk: &BigUint) -> Signature {
        let curve = &self.curve;
        // Get the value "e", which is the hash of message and ID, EC parameters and public key

        let e = BigUint::from_bytes_be(digest);

        // two while loops
        loop {
            // k = rand()
            // (x_1, y_1) = g^kg
            let k = self.curve.random_uint();
            let p_1 = curve.g_mul(&k);
            let (x_1, _) = curve.to_affine(&p_1);
            let x_1 = x_1.to_biguint();

            // r = e + x_1
            let r = (e.clone() + x_1) % curve.get_n();
            if r == BigUint::zero() || r.clone() + k.clone() == curve.get_n() {
                continue;
            }

            // s = (1 + sk)^-1 * (k - r * sk)
            let s1 = curve.inv_n(&(sk.clone() + BigUint::one()));

            let mut s2_1 = r.clone() * sk.clone();
            if s2_1 < k {
                s2_1 = s2_1 + curve.get_n();
            }
            let mut s2 = s2_1 - k;
            s2 = s2 % curve.get_n();
            let s2 = curve.get_n() - s2;

            let s = (s1 * s2) % curve.get_n();

            if s != BigUint::zero() {
                // Output the signature (r, s)
                return Signature { r, s };
            }
            panic!("cannot sign")
        }
    }

    pub fn verify(&self, msg: &[u8], pk: &Point, sig: &Signature) -> bool {
        //Get hash value
        let digest = self.hash("1234567812345678", pk, msg);
        //println!("digest: {:?}", digest);
        self.verify_raw(&digest[..], pk, sig)
    }

    pub fn verify_raw(&self, digest: &[u8], pk: &Point, sig: &Signature) -> bool {
        if digest.len() != 32 {
            panic!("the length of digest must be 32-bytes.");
        }
        let e = BigUint::from_bytes_be(digest);

        let curve = &self.curve;
        // check r and s
        if sig.r == BigUint::zero() || sig.s == BigUint::zero() {
            return false;
        }
        if sig.r >= curve.get_n() || sig.s >= curve.get_n() {
            return false;
        }

        // calculate R
        let t = (sig.s.clone() + sig.r.clone()) % curve.get_n();
        if t == BigUint::zero() {
            return false;
        }

        let p_1 = curve.add(&curve.g_mul(&sig.s), &curve.mul(&t, pk));
        let (x_1, _) = curve.to_affine(&p_1);
        let x_1 = x_1.to_biguint();

        let r_ = (e + x_1) % curve.get_n();

        // check R == r?
        if r_ == sig.r {
            return true;
        }

        return false;
    }

    pub fn new_keypair(&self) -> (Point, BigUint) {
        let curve = &self.curve;
        let mut sk: BigUint = curve.random_uint();
        let mut pk: Point = curve.g_mul(&sk);

        loop {
            if !pk.is_zero() {
                break;
            }
            sk = curve.random_uint();
            pk = curve.g_mul(&sk);
        }

        return (pk, sk);
    }

    pub fn pk_from_sk(&self, sk: &BigUint) -> Point {
        let curve = &self.curve;
        if sk >= &curve.n || sk == &BigUint::zero() {
            panic!("invalid seckey");
        }
        let pk = curve.mul(&sk, &curve.generator());

        return pk;
    }

    pub fn load_pubkey(&self, buf: &[u8]) -> Result<Point, bool> {
        self.curve.bytes_to_point(buf)
    }

    pub fn serialize_pubkey(&self, p: &Point, compress: bool) -> Vec<u8> {
        self.curve.point_to_bytes(p, compress)
    }

    pub fn load_seckey(&self, buf: &[u8]) -> Result<BigUint, bool> {
        if buf.len() != 32 {
            return Err(true);
        }
        let sk = BigUint::from_bytes_be(buf);
        if sk > self.curve.n {
            return Err(true);
        }
        return Ok(sk);
    }

    pub fn serialize_seckey(&self, x: &BigUint) -> Vec<u8> {
        if *x > self.curve.n {
            panic!("invalid secret key");
        }
        let x = FieldElem::from_biguint(x);
        x.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let string = String::from("abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd");
        let msg = string.as_bytes();

        let ctx = SigCtx::new();
        let (pk, sk) = ctx.new_keypair();
        let signature = ctx.sign(msg, &sk, &pk);

        assert!(ctx.verify(msg, &pk, &signature));
    }

    #[test]
    fn test_sig_encode_and_decode() {
        let string = String::from("abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd");
        let msg = string.as_bytes();

        let ctx = SigCtx::new();
        let (pk, sk) = ctx.new_keypair();

        let signature = ctx.sign(msg, &sk, &pk);
        let der = signature.der_encode();
        let sig = Signature::der_decode(&der[..]).unwrap();
        assert!(ctx.verify(msg, &pk, &sig));

        let signature = ctx.sign(msg, &sk, &pk);
        let der = signature.der_encode();
        let sig = Signature::der_decode_raw(&der[2..]).unwrap();
        assert!(ctx.verify(msg, &pk, &sig));
    }

    #[test]
    fn test_key_serialization() {
        let ctx = SigCtx::new();
        let (pk, sk) = ctx.new_keypair();

        let pk_v = ctx.serialize_pubkey(&pk, true);
        let new_pk = ctx.load_pubkey(&pk_v[..]).unwrap();
        assert!(ctx.curve.eq(&new_pk, &pk));

        let sk_v = ctx.serialize_seckey(&sk);
        let new_sk = ctx.load_seckey(&sk_v[..]).unwrap();
        assert_eq!(new_sk, sk);
    }

    #[test]
    fn test_part5_a_1() {
        let message: &[u8] = b"6D65737361676520646967657374";
        let sk = BigUint::from_slice(&[
            0x4DF7C5B8, 0x42FB81EF, 0x2860B51A, 0x88939369, 0xC6D39F95, 0x3F36E38A, 0x7B2144B1,
            0x3945208F,
        ]);

        let curve = EccCtx::new();
        let ctx = SigCtx::new();

        let t = ctx.serialize_seckey(&sk);
        // 3945208f7b2144b13f36e38ac6d39f95889393692860b51a42fb81ef4df7c5b8
        println!("ser => {:?}", hex::encode(t));

        let pk = ctx.pk_from_sk(&sk);
        let raw_pk = curve.point_to_bytes(&pk, false);
        println!("pk => {:?}", hex::encode(raw_pk));
        // x = 09F9DF31 1E5421A1 50DD7D16 1E4BC5C6 72179FAD 1833FC07 6BB08FF3 56F35020
        // y = CCEA490C E26775A5 2DC6EA71 8CC1AA60 0AED05FB F35E084A 6632F607 2DA9AD13


        let sig = ctx.sign(message, &sk, &pk);
        println!("sig => {:?}", sig);
    }

    #[test]
    fn test_gmssl() {
        let msg: &[u8] = &[
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10,
            0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b,
            0x8f, 0x4b, 0xa8, 0xe0,
        ];

        let pk: &[u8] = &[
            4, 233, 185, 71, 125, 111, 174, 63, 105, 217, 19, 218, 72, 114, 185, 96, 243, 176, 1,
            8, 239, 132, 114, 119, 216, 38, 21, 117, 142, 223, 42, 157, 170, 123, 219, 65, 50, 238,
            191, 116, 238, 240, 197, 158, 1, 145, 177, 107, 112, 91, 101, 86, 50, 204, 218, 254,
            172, 2, 250, 33, 56, 176, 121, 16, 215,
        ];

        let sig: &[u8] = &[
            48, 69, 2, 33, 0, 171, 111, 172, 181, 242, 159, 198, 106, 33, 229, 104, 147, 245, 97,
            132, 141, 141, 17, 27, 97, 156, 159, 160, 188, 239, 78, 124, 17, 211, 124, 113, 26, 2,
            32, 53, 21, 4, 195, 198, 42, 71, 17, 110, 157, 113, 185, 178, 74, 147, 87, 129, 179,
            168, 163, 171, 126, 39, 156, 198, 29, 163, 199, 82, 25, 13, 112,
        ];

        let curve = EccCtx::new();
        let ctx = SigCtx::new();

        let pk = curve.bytes_to_point(&pk).unwrap();

        let sig = Signature::der_decode(&sig).unwrap();

        assert!(ctx.verify_raw(msg, &pk, &sig));
    }
}
