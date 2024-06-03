// Yeahbut May 2024

use rsa::PublicKey;
use rsa::{RsaPrivateKey, RsaPublicKey, PaddingScheme, errors::Result};
use rand::{Rng, rngs::OsRng};
use aes::{Aes128, NewBlockCipher};
use aes::cipher::{BlockEncrypt, BlockDecrypt, generic_array::GenericArray};

#[derive(Clone)]
pub struct McCipher {
    pub(crate) key: [u8; 16],
    state_en: u128,
    state_de: u128,
}

impl McCipher {
    pub fn create() -> Self {
        let mut rng = rand::thread_rng();
        let aes_key: [u8; 16] = rng.gen();
        Self {
            key: aes_key.clone(),
            state_en: u128::from_be_bytes(aes_key),
            state_de: u128::from_be_bytes(aes_key),
        }
    }

    pub fn get_encrypted_key(
        &self,
        public_key: &RsaPublicKey,
    ) -> Result<Vec<u8>> {
        encrypt_rsa(public_key, &self.key)
    }

    pub fn create_with_encrypted_key(
        private_key: &RsaPrivateKey,
        data: &[u8],
    ) -> Result<Self> {
        let aes_key: [u8; 16] = decrypt_rsa(private_key, data)?
            .as_slice()[0..16].try_into().unwrap();
        Ok(Self {
            key: aes_key.clone(),
            state_en: u128::from_be_bytes(aes_key),
            state_de: u128::from_be_bytes(aes_key),
        })
    }

    pub fn encrypt_aes(&mut self, data: Vec<u8>) -> Vec<u8> {
        let mut out_data = vec![0; data.len()];
        for i in 0..data.len() {
            out_data[i] = self.encrypt_block(data[i]);
        }
        out_data
    }

    pub fn decrypt_aes(&mut self, data: Vec<u8>) -> Vec<u8> {
        let mut out_data = vec![0; data.len()];
        for i in 0..data.len() {
            out_data[i] = self.decrypt_block(data[i]);
        }
        out_data
    }

    fn encrypt_block(&mut self, data: u8) -> u8 {
        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut block = GenericArray::clone_from_slice(
            &self.state_en.to_be_bytes());
        cipher.encrypt_block(&mut block);
        let data = data ^ block[0];
        self.state_en = (self.state_en << 8) + (data as u128);
        data
    }

    fn decrypt_block(&mut self, data: u8) -> u8 {
        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut block = GenericArray::clone_from_slice(
            &self.state_de.to_be_bytes());
        cipher.decrypt_block(&mut block);
        self.state_de = (self.state_de << 8) + (data as u128);
        let data = data ^ block[0];
        data
    }
}

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
