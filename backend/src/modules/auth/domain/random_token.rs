use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;

pub(in crate::modules::auth) fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
