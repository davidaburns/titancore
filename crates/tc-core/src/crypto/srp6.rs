use crate::crypto::defines::{
    Generator, InterimSessionKey, K, LargeSafePrime, PasswordVerifier, PrivateKey, ProofKey,
    PublicKey, ReconnectSeed, Salt, SessionKey, Sha1Hash, XorHash,
};
use hmac::digest::Update;
use sha1::{Digest, Sha1};

pub fn calculate_x(username: &str, password: &str, salt: &Salt) -> Sha1Hash {
    let p = Sha1::new()
        .chain_update(username)
        .chain_update(":")
        .chain_update(password)
        .finalize();

    let x = Sha1::new()
        .chain_update(salt.as_bytes_le())
        .chain_update(p)
        .finalize();

    Sha1Hash::from_bytes_le(&x.into())
}

pub fn calculate_u(client_public_key: &PublicKey, server_public_key: &PublicKey) -> Sha1Hash {
    let hash = Sha1::new()
        .chain(client_public_key.as_bytes_le())
        .chain(server_public_key.as_bytes_le())
        .finalize();

    Sha1Hash::from_bytes_le(&hash.into())
}

pub fn calculate_xor_hash(lsp: &LargeSafePrime, g: &Generator) -> Sha1Hash {
    let lsp_hash = Sha1::new().chain_update(lsp.as_bytes_le()).finalize();
    let g_hash = Sha1::new().chain_update([g.value()]).finalize();

    let mut xor_hash = [0u8; XorHash::SIZE];
    for (i, n) in lsp_hash.iter().enumerate() {
        xor_hash[i] = *n ^ g_hash[i];
    }

    Sha1Hash::from_bytes_le(&xor_hash)
}

pub fn calculate_password_verifier(
    username: &str,
    password: &str,
    salt: &Salt,
    g: &Generator,
    lsp: &LargeSafePrime,
) -> PasswordVerifier {
    let x = calculate_x(username, password, salt).to_bigint();
    let g = g.to_bigint();
    let lsp = lsp.to_bigint();

    g.modpow(&x, &lsp).into()
}

pub fn calculate_client_public_key(
    client_private_key: &PrivateKey,
    g: &Generator,
    lsp: &LargeSafePrime,
) -> PublicKey {
    let cpk = client_private_key.to_bigint();
    let g = g.to_bigint();
    let lsp = lsp.to_bigint();

    g.modpow(&cpk, &lsp).into()
}

pub fn calculate_server_public_key(
    verifier: &PasswordVerifier,
    server_private_key: &PrivateKey,
    g: &Generator,
    lsp: &LargeSafePrime,
) -> PublicKey {
    let g = g.to_bigint();
    let lsp = lsp.to_bigint();
    let server_public_key = (K::default().to_bigint() * verifier.to_bigint()
        + g.modpow(&server_private_key.to_bigint(), &lsp))
        % lsp;

    server_public_key.into()
}

pub fn calculate_client_s(
    client_private_key: &PrivateKey,
    server_public_key: &PublicKey,
    x: &Sha1Hash,
    u: &Sha1Hash,
    k: &K,
    g: &Generator,
    lsp: &LargeSafePrime,
) -> InterimSessionKey {
    let k = k.to_bigint();
    let g = g.to_bigint();
    let lsp = lsp.to_bigint();

    let cpk = client_private_key.to_bigint();
    let spk = server_public_key.to_bigint();
    let x = x.to_bigint();
    let u = u.to_bigint();

    (spk - (k * g.modpow(&x, &lsp)))
        .modpow(&(cpk + u * x), &lsp)
        .into()
}

pub fn calculate_server_s(
    client_public_key: &PublicKey,
    server_private_key: &PrivateKey,
    verifier: &PasswordVerifier,
    u: &Sha1Hash,
    lsp: &LargeSafePrime,
) -> InterimSessionKey {
    let lsp = lsp.to_bigint();
    let cpk = client_public_key.to_bigint();
    let spk = server_private_key.to_bigint();
    let v = verifier.to_bigint();
    let u = u.to_bigint();

    (cpk * v.modpow(&u, &lsp)).modpow(&spk, &lsp).into()
}

pub fn calculate_server_session_key(
    client_public_key: &PublicKey,
    server_public_key: &PublicKey,
    server_private_key: &PrivateKey,
    verifier: &PasswordVerifier,
    lsp: &LargeSafePrime,
) -> SessionKey {
    let u = calculate_u(client_public_key, server_public_key);
    let s = calculate_server_s(client_public_key, server_private_key, verifier, &u, lsp);

    sha1_interleaved(s)
}

pub fn calculate_client_session_key(
    username: &str,
    password: &str,
    server_public_key: &PublicKey,
    client_public_key: &PublicKey,
    client_private_key: &PrivateKey,
    salt: &Salt,
    k: &K,
    g: &Generator,
    lsp: &LargeSafePrime,
) -> SessionKey {
    let x = calculate_x(username, password, salt);
    let u = calculate_u(client_public_key, server_public_key);
    let s = calculate_client_s(client_private_key, server_public_key, &x, &u, k, g, lsp);

    sha1_interleaved(s)
}

pub fn calculate_server_proof(
    client_public_key: &PublicKey,
    client_proof: &ProofKey,
    session_key: &SessionKey,
) -> ProofKey {
    let hashed = Sha1::new()
        .chain_update(client_public_key.as_bytes_le())
        .chain_update(client_proof.as_bytes_le())
        .chain_update(session_key.as_bytes_le())
        .finalize();

    ProofKey::from_bytes_le(&hashed.into())
}

pub fn calculate_client_proof(
    xor_hash: &XorHash,
    username: &str,
    session_key: &SessionKey,
    client_public_key: &PublicKey,
    server_public_key: &PublicKey,
    salt: &Salt,
) -> ProofKey {
    let username_hash = Sha1::new().chain(username).finalize();
    let proof_hash = Sha1::new()
        .chain_update(xor_hash.as_bytes_le())
        .chain_update(username_hash)
        .chain_update(salt.as_bytes_le())
        .chain_update(client_public_key.as_bytes_le())
        .chain_update(server_public_key.as_bytes_le())
        .chain_update(session_key.as_bytes_le())
        .finalize()
        .into();

    ProofKey::from_bytes_le(&proof_hash)
}

pub fn calculate_reconnect_proof(
    username: &str,
    client_seed: &ReconnectSeed,
    server_seed: &ReconnectSeed,
    session_key: &SessionKey,
) -> ProofKey {
    let hash = Sha1::new()
        .chain_update(username)
        .chain_update(client_seed.as_bytes_le())
        .chain_update(server_seed.as_bytes_le())
        .chain_update(session_key.as_bytes_le())
        .finalize()
        .into();

    ProofKey::from_bytes_le(&hash)
}

fn sha1_interleaved(s_key: InterimSessionKey) -> SessionKey {
    let s = s_key.as_split_slice();
    let e: Vec<u8> = s
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, &byte)| byte)
        .collect();

    let f: Vec<u8> = s
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, &byte)| byte)
        .collect();

    let g = Sha1::new().chain_update(&e).finalize();
    let h = Sha1::new().chain_update(&f).finalize();

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
    use crate::crypto::defines::{
        Generator, InterimSessionKey, K, LargeSafePrime, PasswordVerifier, PrivateKey, ProofKey,
        PublicKey, ReconnectSeed, Salt, SessionKey, Sha1Hash, XorHash,
    };
    use crate::crypto::srp6::{
        calculate_client_proof, calculate_client_public_key, calculate_client_s,
        calculate_client_session_key, calculate_password_verifier, calculate_reconnect_proof,
        calculate_server_proof, calculate_server_public_key, calculate_server_s,
        calculate_server_session_key, calculate_u, calculate_x, calculate_xor_hash,
        sha1_interleaved,
    };

    #[test]
    fn test_calculate_x() {
        let tests = include_str!("../../tests/srp6/calculate_x_salt_values.txt");
        let username = "USERNAME123";
        let password = "PASSWORD123";

        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let salt = Salt::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();

            let x = calculate_x(username, password, &salt);

            assert_eq!(expected, x);
        }
    }

    #[test]
    fn test_calculate_x_static_salts() {
        let tests = include_str!("../../tests/srp6/calculate_x_values.txt");
        let salt = Salt::from_hex_str_be(
            "CAC94AF32D817BA64B13F18FDEDEF92AD4ED7EF7AB0E19E9F2AE13C828AEAF57",
        )
        .unwrap();

        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let username = line.next().unwrap();
            let password = line.next().unwrap();
            let expected = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();

            let x = calculate_x(username, password, &salt);

            assert_eq!(expected, x);
        }
    }

    #[test]
    fn test_calculate_u() {
        let tests = include_str!("../../tests/srp6/calculate_u_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();

            let u = calculate_u(&client_public_key, &server_public_key);

            assert_eq!(expected, u);
        }
    }

    #[test]
    fn test_calculate_password_verifier() {
        let tests = include_str!("../../tests/srp6/calculate_v_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let username = line.next().unwrap();
            let password = line.next().unwrap();
            let salt = Salt::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = PasswordVerifier::from_hex_str_be(line.next().unwrap()).unwrap();

            let v = calculate_password_verifier(
                username,
                password,
                &salt,
                &Generator::default(),
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, v);
        }
    }

    #[test]
    fn test_calculate_client_public_key() {
        let tests = include_str!("../../tests/srp6/calculate_A_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let client_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let client_public_key = calculate_client_public_key(
                &client_private_key,
                &Generator::default(),
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, client_public_key);
        }
    }

    #[test]
    fn test_calculate_server_public_key() {
        let tests = include_str!("../../tests/srp6/calculate_B_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let v = PasswordVerifier::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let server_public_key = calculate_server_public_key(
                &v,
                &server_private_key,
                &Generator::default(),
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, server_public_key);
        }
    }

    #[test]
    fn test_calculate_client_s() {
        let tests = include_str!("../../tests/srp6/calculate_client_S_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let server_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let client_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let x = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();
            let u = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = InterimSessionKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let s = calculate_client_s(
                &client_private_key,
                &server_public_key,
                &x,
                &u,
                &K::default(),
                &Generator::default(),
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, s);
        }
    }

    #[test]
    fn test_calculate_server_s() {
        let tests = include_str!("../../tests/srp6/calculate_server_S_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let v = PasswordVerifier::from_hex_str_be(line.next().unwrap()).unwrap();
            let u = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = InterimSessionKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let s = calculate_server_s(
                &client_public_key,
                &server_private_key,
                &v,
                &u,
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, s);
        }
    }

    #[test]
    fn test_calculate_server_session_key() {
        let tests = include_str!("../../tests/srp6/calculate_server_session_key.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let v = PasswordVerifier::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let server_public_key = calculate_server_public_key(
                &v,
                &server_private_key,
                &Generator::default(),
                &LargeSafePrime::default(),
            );

            let session_key = calculate_server_session_key(
                &client_public_key,
                &server_public_key,
                &server_private_key,
                &v,
                &LargeSafePrime::default(),
            );

            assert_eq!(expected, session_key)
        }
    }

    #[test]
    fn test_calculate_client_session_key() {
        let tests = include_str!("../../tests/srp6/calculate_client_session_key.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();

            let username = line.next().unwrap().to_uppercase();
            let password = line.next().unwrap().to_uppercase();
            let server_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let client_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let g = Generator::from_value(line.next().unwrap().parse::<u8>().unwrap());
            let lsp = LargeSafePrime::from_hex_str_be(line.next().unwrap()).unwrap();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let salt = Salt::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let session_key = calculate_client_session_key(
                username.as_str(),
                password.as_str(),
                &server_public_key,
                &client_public_key,
                &client_private_key,
                &salt,
                &K::default(),
                &g,
                &lsp,
            );

            assert_eq!(expected, session_key);
        }
    }

    #[test]
    fn test_calculate_server_proof() {
        let tests = include_str!("../../tests/srp6/calculate_M2_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let client_proof = ProofKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let session_key = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = ProofKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let server_proof =
                calculate_server_proof(&client_public_key, &client_proof, &session_key);

            assert_eq!(expected, server_proof);
        }
    }

    #[test]
    fn test_calculate_client_proof() {
        let tests = include_str!("../../tests/srp6/calculate_M1_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let username = line.next().unwrap();
            let session_key = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let client_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_public_key = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let salt = Salt::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = ProofKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let client_proof = calculate_client_proof(
                &XorHash::default(),
                username,
                &session_key,
                &client_public_key,
                &server_public_key,
                &salt,
            );

            assert_eq!(expected, client_proof);
        }
    }

    #[test]
    fn test_calculate_xor_hash() {
        let tests = include_str!("../../tests/srp6/calculate_xor_hash.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let g = Generator::from_value(line.next().unwrap().parse::<u8>().unwrap());
            let lsp = LargeSafePrime::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = Sha1Hash::from_hex_str_be(line.next().unwrap()).unwrap();

            let xor_hash = calculate_xor_hash(&lsp, &g);

            assert_eq!(expected, xor_hash);
        }
    }

    #[test]
    fn test_calculate_reconnect_proof() {
        let tests = include_str!("../../tests/srp6/calculate_reconnection_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let username = line.next().unwrap();
            let client_seed = ReconnectSeed::from_hex_str_be(line.next().unwrap()).unwrap();
            let server_seed = ReconnectSeed::from_hex_str_be(line.next().unwrap()).unwrap();
            let session_key = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = ProofKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let reconnect_proof =
                calculate_reconnect_proof(username, &client_seed, &server_seed, &session_key);

            assert_eq!(expected, reconnect_proof);
        }
    }

    #[test]
    fn test_split_key() {
        let tests = include_str!("../../tests/srp6/calculate_split_s_key.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let s = InterimSessionKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = hex::decode(line.next().unwrap()).unwrap();

            let mut s = s.as_split_slice().to_vec();
            s.reverse();

            assert_eq!(expected, s);
        }
    }

    #[test]
    fn test_sha1_interleaved() {
        let tests = include_str!("../../tests/srp6/calculate_interleaved_values.txt");
        for line in tests.lines() {
            let mut line = line.split_whitespace();
            let s = InterimSessionKey::from_hex_str_be(line.next().unwrap()).unwrap();
            let expected = SessionKey::from_hex_str_be(line.next().unwrap()).unwrap();

            let interleaved = sha1_interleaved(s);

            assert_eq!(expected, interleaved);
        }
    }
}
