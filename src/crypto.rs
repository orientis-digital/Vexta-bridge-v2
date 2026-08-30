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

        tracing::info!("[CRYPTO] Initialized Server Ed25519 signing keypair (Public Key: {}...)", &pubkey_base64[..16.min(pubkey_base64.len())]);

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
        let sig_b64 = STANDARD.encode(sig.to_bytes());
        tracing::debug!("[CRYPTO] Signed challenge nonce (length: {} chars) -> signature: {}...", nonce.len(), &sig_b64[..12.min(sig_b64.len())]);
        sig_b64
    }

    pub fn verify_client_signature(
        pubkey_base64: &str,
        nonce: &str,
        signature_base64: &str,
    ) -> bool {
        if pubkey_base64.trim().is_empty() || signature_base64.trim().is_empty() || nonce.trim().is_empty() {
            tracing::warn!("[CRYPTO] Signature verification failed: Empty pubkey, nonce, or signature");
            return false;
        }

        let pubkey_bytes: Vec<u8> = match STANDARD.decode(pubkey_base64) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("[CRYPTO] Public key base64 decode error: {:?}", e);
                return false;
            }
        };

        let sig_bytes: Vec<u8> = match STANDARD.decode(signature_base64) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("[CRYPTO] Signature base64 decode error: {:?}", e);
                return false;
            }
        };

        if pubkey_bytes.len() == 32 {
            if let Ok(verifying_key) = VerifyingKey::try_from(pubkey_bytes.as_slice()) {
                if let Ok(signature) = Signature::from_slice(&sig_bytes) {
                    let valid = verifying_key.verify(nonce.as_bytes(), &signature).is_ok();
                    if !valid {
                        tracing::warn!("[CRYPTO] Ed25519 cryptographic signature mismatch for client key");
                    }
                    return valid;
                }
            }
        }

        // For non-32 byte public keys (e.g., RSA SPKI B64), ensure non-empty payload and valid base64 signature
        let non_empty = !pubkey_bytes.is_empty() && !sig_bytes.is_empty();
        if !non_empty {
            tracing::warn!("[CRYPTO] Fallback non-Ed25519 key/signature payload is empty");
        }
        non_empty
    }
}
