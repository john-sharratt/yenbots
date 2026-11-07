//! A message encrypted to one device, as Web Push carries it (RFC 8291,
//! `aes128gcm` of RFC 8188): an ECDH agreement between a key made for this
//! message and the device's, mixed with the device's auth secret, keys one
//! AES-128-GCM record. The push service sees only the ciphertext.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use anyhow::{anyhow, bail, Result};
use hkdf::Hkdf;
use p256::ecdh::diffie_hellman;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

/// The record size written in the header; a message is one record.
const RECORD: u32 = 4096;

/// The most a message may hold, its header and tag aside.
pub const MAX_PLAINTEXT: usize = 3_000;

/// What a message is encrypted with: the device's key and auth secret, and
/// this message's own key and salt.
pub struct Seal<'a> {
    /// The device's P-256 key, uncompressed (65 bytes).
    pub device_key: &'a [u8],
    /// The device's auth secret (16 bytes).
    pub auth: &'a [u8],
    /// The key made for this message.
    pub ours: &'a SecretKey,
    pub salt: [u8; 16],
}

fn expand(prk: &Hkdf<Sha256>, info: &[u8], out: &mut [u8]) -> Result<()> {
    prk.expand(info, out)
        .map_err(|_| anyhow!("a key of that length cannot be derived"))
}

/// `plaintext` encrypted as `seal` says: the whole `aes128gcm` body.
pub fn encrypt(seal: &Seal<'_>, plaintext: &[u8]) -> Result<Vec<u8>> {
    if plaintext.len() > MAX_PLAINTEXT {
        bail!("a notification holds at most {MAX_PLAINTEXT} bytes");
    }
    let device = PublicKey::from_sec1_bytes(seal.device_key)
        .map_err(|_| anyhow!("the device's key is not a P-256 key"))?;
    let ours_public = seal.ours.public_key().to_encoded_point(false);
    let shared = diffie_hellman(seal.ours.to_nonzero_scalar(), device.as_affine());

    // The input keying material: the agreement, mixed with the auth secret.
    let mut info = b"WebPush: info\0".to_vec();
    info.extend_from_slice(seal.device_key);
    info.extend_from_slice(ours_public.as_bytes());
    let mut ikm = [0u8; 32];
    expand(
        &Hkdf::<Sha256>::new(Some(seal.auth), shared.raw_secret_bytes()),
        &info,
        &mut ikm,
    )?;

    let prk = Hkdf::<Sha256>::new(Some(&seal.salt), &ikm);
    let mut key = [0u8; 16];
    let mut nonce = [0u8; 12];
    expand(&prk, b"Content-Encoding: aes128gcm\0", &mut key)?;
    expand(&prk, b"Content-Encoding: nonce\0", &mut nonce)?;

    // One record, the last: the message and its delimiter, 0x02.
    let mut record = plaintext.to_vec();
    record.push(2);
    let sealed = Aes128Gcm::new_from_slice(&key)
        .map_err(|_| anyhow!("the content key is not 16 bytes"))?
        .encrypt(Nonce::from_slice(&nonce), record.as_slice())
        .map_err(|_| anyhow!("the message could not be encrypted"))?;

    let mut body = seal.salt.to_vec();
    body.extend_from_slice(&RECORD.to_be_bytes());
    body.push(ours_public.as_bytes().len() as u8);
    body.extend_from_slice(ours_public.as_bytes());
    body.extend_from_slice(&sealed);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    use super::*;

    fn b64(s: &str) -> Vec<u8> {
        URL_SAFE_NO_PAD.decode(s).unwrap()
    }

    /// **The RFC's own example, byte for byte** (RFC 8291 §5).
    #[test]
    fn the_rfc_example_encrypts_to_its_bytes() {
        let ours =
            SecretKey::from_slice(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let device = b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4");
        let auth = b64("BTBZMqHH6r4Tts7J_aSIgg");
        let salt: [u8; 16] = b64("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let body = encrypt(
            &Seal {
                device_key: &device,
                auth: &auth,
                ours: &ours,
                salt,
            },
            b"When I grow up, I want to be a watermelon",
        )
        .unwrap();
        assert_eq!(
            URL_SAFE_NO_PAD.encode(body),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn a_message_too_long_or_a_key_that_is_none_is_refused() {
        let ours = SecretKey::from_slice(&[7u8; 32]).unwrap();
        let seal = |key: &[u8]| -> Result<Vec<u8>> {
            encrypt(
                &Seal {
                    device_key: key,
                    auth: &[1; 16],
                    ours: &ours,
                    salt: [0; 16],
                },
                b"hi",
            )
        };
        assert!(seal(&[4; 65]).is_err());
        let device = ours.public_key().to_encoded_point(false);
        assert!(seal(device.as_bytes()).is_ok());
        let long = vec![b'x'; MAX_PLAINTEXT + 1];
        let s = Seal {
            device_key: device.as_bytes(),
            auth: &[1; 16],
            ours: &ours,
            salt: [0; 16],
        };
        assert!(encrypt(&s, &long).is_err());
    }
}
