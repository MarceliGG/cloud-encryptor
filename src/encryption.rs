use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{AeadCore, AeadInPlace, KeyInit, OsRng},
    Aes256Gcm,
};

pub fn encrypt(data: Vec<u8>, key: &Vec<u8>) -> Vec<u8> {
    let mut d = data;
    let n = &Aes256Gcm::generate_nonce(&mut OsRng);
    Aes256Gcm::new_from_slice(key)
        .unwrap()
        .encrypt_in_place(n, b"", &mut d);
    let mut out = n.to_vec();
    out.append(&mut d);
    out
}

pub fn decrypt(data: Vec<u8>, key: &Vec<u8>) -> Vec<u8> {
    let mut out = data;
    let nonce = GenericArray::from([
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
        out.remove(0),
    ]);
    Aes256Gcm::new_from_slice(key)
        .unwrap()
        .decrypt_in_place(&nonce, b"", &mut out);
    out
}
