// Yeahbut May 2024

use rsa::PublicKey;
use rsa::{RsaPrivateKey, RsaPublicKey, PaddingScheme, errors::Result};
use rand::{Rng, rngs::OsRng};
use aes::{Aes128, NewBlockCipher};
use aes::cipher::{BlockEncrypt, BlockDecrypt, generic_array::GenericArray};

#[derive(Clone)]
pub struct McCipher {
    key: [u8; 16],
    state_en: [u8; 16],
    state_de: [u8; 16],
}

impl McCipher {
    pub fn create() -> Self {
        let mut rng = rand::thread_rng();
        let aes_key: [u8; 16] = rng.gen();
        Self {
            key: aes_key.clone(),
            state_en: aes_key.clone(),
            state_de: aes_key.clone(),
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
            state_en: aes_key.clone(),
            state_de: aes_key.clone(),
        })
    }

    pub fn encrypt_aes(&self, data: Vec<u8>) -> Vec<u8> {
        let mut out_data = vec![0; data.len()];
        for i in 0..data.len() {
            out_data[i] = self.encrypt_block(data[i]);
        }
        out_data
    }

    pub fn decrypt_aes(&self, data: Vec<u8>) -> Vec<u8> {
        let mut out_data = vec![0; data.len()];
        for i in 0..data.len() {
            out_data[i] = self.decrypt_block(data[i]);
        }
        out_data
    }

    fn shift_left(mut arr: [u8; 16], new: u8) {
        for i in 1..arr.len() {
            arr[i] = arr[i - 1];
        }
        arr[0] = new;
    }

    fn encrypt_block(&self, data: u8) -> u8 {
        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut block = GenericArray::clone_from_slice(&self.state_en);
        cipher.encrypt_block(&mut block);
        let data = data ^ block[15];
        Self::shift_left(self.state_en, data);
        data
    }

    fn decrypt_block(&self, data: u8) -> u8 {
        let cipher = Aes128::new(GenericArray::from_slice(&self.key));
        let mut block = GenericArray::clone_from_slice(&self.state_en);
        cipher.decrypt_block(&mut block);
        Self::shift_left(self.state_de, data);
        let data = data ^ block[15];
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
