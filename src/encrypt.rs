// Yeahbut May 2024

use rsa::PublicKey;
use rsa::{RsaPrivateKey, RsaPublicKey, PaddingScheme, errors::Result};
use rand::rngs::OsRng;
use aes::Aes128;
use aes::cipher::{
    BlockEncrypt, BlockDecrypt, NewBlockCipher, generic_array::GenericArray};

pub fn generate_rsa_keys() -> Result<RsaPrivateKey> {
    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;
    Ok(private_key)
}

pub fn encrypt_rsa(
    public_key: &RsaPublicKey,
    data: &[u8; 16],
) -> Result<Vec<u8>> {
    let padding = PaddingScheme::new_pkcs1v15_encrypt();
    let mut rng = OsRng;
    public_key.encrypt(&mut rng, padding, data)
}

pub fn decrypt_rsa(
    private_key: &RsaPrivateKey,
    data: &[u8],
) -> Result<Vec<u8>> {
    let padding = PaddingScheme::new_pkcs1v15_encrypt();
    private_key.decrypt(padding, data)
}

pub fn encrypt_aes(key: &[u8; 16], data: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(data);
    cipher.encrypt_block(&mut block);
    block.to_vec()
}

pub fn decrypt_aes(key: &[u8; 16], data: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(data);
    cipher.decrypt_block(&mut block);
    block.to_vec()
}
