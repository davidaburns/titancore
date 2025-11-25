mod inner_hex;

use core::crypto::defines::{
    Generator, K, LargeSafePrime, PasswordVerifier, PrivateKey, PublicKey,
};
use core::crypto::error::InvalidPublicKeyError;

pub fn calculate_server_public_key(
    verifier: PasswordVerifier,
    server_private_key: PrivateKey,
) -> Result<PublicKey, InvalidPublicKeyError> {
    let g = Generator::default().to_bigint();
    let lsp = LargeSafePrime::default().to_bigint();
    let server_public_key = (K::default().to_bigint() * verifier.to_bigint()
        + g.modpow(&server_private_key.to_bigint(), &lsp))
        % lsp;

    PublicKey::try_from_bigint(server_public_key)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Failed tests:
    // B71890DA4E0EAA2E0C3F3BCEAAA1D5DE5B2FB11A9ADDBEFDDCE6740480D99F62 9BB0CFDDCCC8CFF426AFDF4C1107A849D8FDAF8F3E5CF716FB974FE024917C 7E6C1983785D2287C9AF52610FDD86CB869E811E174179BBB296946A1E416718
    // FAD6A2ED3C4BECEDCBB67CD2D1921599FE0F4805B76768BD5EDB58903DCAB6 FAECEDEE9AD139D896B9A30AE7BF4F757301D8F15B72332ABF079E5BBC2DBD7B 66A17CECE10A907332D8E15C5504A966E2EC8A4349838B2CB738BA31743D8D18

    // let v = PasswordVerifier::from_hex_str_be(
    //     "B71890DA4E0EAA2E0C3F3BCEAAA1D5DE5B2FB11A9ADDBEFDDCE6740480D99F62",
    // )
    // .unwrap();
    // let server_private_key = PrivateKey::from_hex_str_be(
    //     "9BB0CFDDCCC8CFF426AFDF4C1107A849D8FDAF8F3E5CF716FB974FE024917C",
    // )
    // .unwrap();
    // let expected = PublicKey::from_hex_str_be(
    //     "7E6C1983785D2287C9AF52610FDD86CB869E811E174179BBB296946A1E416718",
    // )
    // .unwrap();

    // let server_public_key = calculate_server_public_key(v, server_private_key)?;
    // println!("{:?}", expected.to_hex_str());
    // println!("{:?}", server_public_key.to_hex_str());

    let mut idx = 1;
    let tests = include_str!("../../core/tests/srp6/calculate_B_values.txt");
    for line in tests.lines() {
        let mut line = line.split_whitespace();
        let v = PasswordVerifier::from_hex_str_be(line.next().unwrap()).unwrap();
        let server_private_key = PrivateKey::from_hex_str_be(line.next().unwrap()).unwrap();
        let expected = PublicKey::from_hex_str_be(line.next().unwrap()).unwrap();

        let server_public_key = calculate_server_public_key(v, server_private_key)?;
        if expected != server_public_key {
            println!("idx={idx}");
            println!("{:?}", expected.to_hex_str());
            println!("{:?}\n", server_public_key.to_hex_str());
        }

        idx += 1;
    }

    Ok(())
}
