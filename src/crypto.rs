use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use rand::rngs::OsRng;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

#[allow(dead_code)]
pub struct ServerCrypto {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub pubkey_base64: String,
}

impl ServerCrypto {
    pub fn new_or_generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let pubkey_bytes = verifying_key.to_bytes();
        let pubkey_base64 = STANDARD.encode(pubkey_bytes);

        Self {
            signing_key,
            verifying_key,
            pubkey_base64,
        }
    }

    pub fn generate_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
        nonce
    }

    pub fn get_pubkey_pem(&self) -> String {
        self.pubkey_base64.clone()
    }

    pub fn sign_nonce(&self, nonce: &str) -> String {
        let sig = self.signing_key.sign(nonce.as_bytes());
        STANDARD.encode(sig.to_bytes())
    }

    pub fn verify_client_signature(
        pubkey_base64: &str,
        nonce: &str,
        signature_base64: &str,
    ) -> bool {
        let pubkey_bytes: Vec<u8> = match STANDARD.decode(pubkey_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let sig_bytes: Vec<u8> = match STANDARD.decode(signature_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let verifying_key = match VerifyingKey::try_from(pubkey_bytes.as_slice()) {
            Ok(k) => k,
            Err(_) => return false,
        };

        let signature = match Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        verifying_key.verify(nonce.as_bytes(), &signature).is_ok()
    }
}
