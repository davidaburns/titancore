use crate::crypto::defines::{
    Generator, InterimSessionKey, K, LargeSafePrime, PasswordVerifier, PrecalculatedXorHash,
    PrivateKey, ProofKey, PublicKey, ReconnectSeed, Salt, SessionKey, Sha1Hash,
};
use hmac::digest::Update;
use sha1::{Digest, Sha1};

pub fn calculate_x(username: &str, password: &str, salt: Salt) -> Sha1Hash {
    let unp = format!("{}:{}", username.to_uppercase(), password.to_uppercase());
    let p = Sha1::new().chain(unp.as_bytes()).finalize();
    let x = Sha1::new().chain(salt.as_bytes_le()).chain(p).finalize();

    Sha1Hash::from_bytes_le(&x.into())
}

pub fn calculate_u(client_public_key: PublicKey, server_public_key: PublicKey) -> Sha1Hash {
    let hash = Sha1::new()
        .chain(client_public_key.as_bytes_le())
        .chain(server_public_key.as_bytes_le())
        .finalize();

    Sha1Hash::from_bytes_le(&hash.into())
}

pub fn calculate_password_verifier(username: &str, password: &str, salt: Salt) -> PasswordVerifier {
    let x = calculate_x(username, password, salt).to_bigint();
    let g = Generator::default().to_bigint();
    let lsp = LargeSafePrime::default().to_bigint();

    g.modpow(&x, &lsp).into()
}

pub fn calculate_client_public_key(client_private_key: PrivateKey) -> PublicKey {
    let g = Generator::default().to_bigint();
    let lsp = LargeSafePrime::default().to_bigint();
    let cpk = client_private_key.to_bigint();

    g.modpow(&cpk, &lsp).into()
}

pub fn calculate_server_public_key(
    verifier: PasswordVerifier,
    server_private_key: PrivateKey,
) -> PublicKey {
    let verifier = verifier.to_bigint();
    let server_private_key = server_private_key.to_bigint();
    let k = K::default().to_bigint();
    let g = Generator::default().to_bigint();
    let lsp = LargeSafePrime::default().to_bigint();

    let interim = k * verifier + g.modpow(&server_private_key, &lsp);
    (interim % lsp).into()
}

pub fn calculate_client_s(
    client_private_key: PrivateKey,
    server_public_key: PublicKey,
    x: Sha1Hash,
    u: Sha1Hash,
) -> InterimSessionKey {
    let k = K::default().to_bigint();
    let g = Generator::default().to_bigint();
    let lsp = LargeSafePrime::default().to_bigint();

    let cpk = client_private_key.to_bigint();
    let spk = server_public_key.to_bigint();
    let x = x.to_bigint();
    let u = u.to_bigint();

    (spk - (k * g.modpow(&x, &lsp)))
        .modpow(&(cpk + u * x), &lsp)
        .into()
}

pub fn calculate_server_s(
    client_public_key: PublicKey,
    server_private_key: PrivateKey,
    verifier: PasswordVerifier,
    u: Sha1Hash,
) -> InterimSessionKey {
    let lsp = LargeSafePrime::default().to_bigint();
    let cpk = client_public_key.to_bigint();
    let spk = server_private_key.to_bigint();
    let v = verifier.to_bigint();
    let u = u.to_bigint();

    (cpk * v.modpow(&u, &lsp)).modpow(&spk, &lsp).into()
}

pub fn calculate_server_session_key(
    client_public_key: PublicKey,
    server_public_key: PublicKey,
    server_private_key: PrivateKey,
    verifier: PasswordVerifier,
) -> SessionKey {
    let u = calculate_u(client_public_key, server_public_key);
    let s = calculate_server_s(client_public_key, server_private_key, verifier, u);

    sha1_interleaved(s)
}

pub fn calculate_client_session_key(
    username: &str,
    password: &str,
    server_public_key: PublicKey,
    client_public_key: PublicKey,
    client_private_key: PrivateKey,
    salt: Salt,
) -> SessionKey {
    let x = calculate_x(username, password, salt);
    let u = calculate_u(client_public_key, server_public_key);
    let s = calculate_client_s(client_private_key, server_public_key, x, u);

    sha1_interleaved(s)
}

pub fn calculate_server_proof(
    client_public_key: PublicKey,
    client_proof: ProofKey,
    session_key: SessionKey,
) -> ProofKey {
    let hashed = Sha1::new()
        .chain(client_public_key.as_bytes_le())
        .chain(client_proof.as_bytes_le())
        .chain(session_key.as_bytes_le())
        .finalize();

    ProofKey::from_bytes_le(&hashed.into())
}

pub fn calculate_client_proof(
    username: &str,
    session_key: SessionKey,
    client_public_key: PublicKey,
    server_public_key: PublicKey,
    salt: Salt,
) -> ProofKey {
    let username_hash = Sha1::new().chain(username).finalize();
    let proof_hash = Sha1::new()
        .chain(PrecalculatedXorHash::default().as_bytes_le())
        .chain(username_hash)
        .chain(salt.as_bytes_le())
        .chain(client_public_key.as_bytes_le())
        .chain(server_public_key.as_bytes_le())
        .chain(session_key.as_bytes_le())
        .finalize()
        .into();

    ProofKey::from_bytes_le(&proof_hash)
}

pub fn calculate_reconnect_proof(
    username: &str,
    client_seed: ReconnectSeed,
    server_seed: ReconnectSeed,
    session_key: SessionKey,
) -> ProofKey {
    let hash = Sha1::new()
        .chain(username)
        .chain(client_seed.as_bytes_le())
        .chain(server_seed.as_bytes_le())
        .chain(session_key.as_bytes_le())
        .finalize()
        .into();

    ProofKey::from_bytes_le(&hash)
}

fn split_key(s_key: InterimSessionKey) -> InterimSessionKey {
    let bytes = *s_key.as_bytes_le();
    let mut slice = &bytes[..];
    while slice.len() >= 2 && slice[0] == 0x00 {
        slice = &slice[2..];
    }

    let mut result = [0u8; InterimSessionKey::SIZE];
    result[0..slice.len()].copy_from_slice(&slice);
    result.into()
}

fn sha1_interleaved(s_key: InterimSessionKey) -> SessionKey {
    let s = split_key(s_key);
    let s = s.to_vec();

    let e: Vec<u8> = s
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, &byte)| byte)
        .collect();

    let f: Vec<u8> = s
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, &byte)| byte)
        .collect();

    let g = Sha1::new().chain(&e).finalize();
    let h = Sha1::new().chain(&f).finalize();

    let mut result = Vec::new();
    let zip = g.iter().zip(h.iter());
    for r in zip {
        result.push(*r.0);
        result.push(*r.1);
    }

    let result = <[u8; SessionKey::SIZE]>::try_from(result).unwrap();
    result.into()
}

#[cfg(test)]
mod test {

    #[ignore]
    #[test]
    fn test_calculate_x() {
        // let username = "USERNAME123";
        // let password = "PASSWORD123";
    }

    #[ignore]
    #[test]
    fn test_calculate_u() {}

    #[ignore]
    #[test]
    fn test_calculate_password_verifier() {}

    #[ignore]
    #[test]
    fn test_calculate_client_public_key() {}

    #[ignore]
    #[test]
    fn test_calculate_server_public_key() {}

    #[ignore]
    #[test]
    fn test_calculate_client_s() {}

    #[ignore]
    #[test]
    fn test_calculate_server_s() {}

    #[ignore]
    #[test]
    fn test_calculate_server_session_key() {}

    #[ignore]
    #[test]
    fn test_calculate_client_session_key() {}

    #[ignore]
    #[test]
    fn test_calculate_server_proof() {}

    #[ignore]
    #[test]
    fn test_calculate_client_proof() {}

    #[ignore]
    #[test]
    fn test_calculate_reconnect_proof() {}

    #[ignore]
    #[test]
    fn test_split_key() {}

    #[ignore]
    #[test]
    fn test_sha1_interleaved() {}
}
