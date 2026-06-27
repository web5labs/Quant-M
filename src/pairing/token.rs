use ring::digest;

pub fn generate_token() -> String {
    let bytes: [u8; 24] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn hash_token(token: &str) -> String {
    let digest = digest::digest(&digest::SHA256, token.as_bytes());
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn short_hash(value: &str) -> String {
    hash_token(value).chars().take(16).collect()
}
