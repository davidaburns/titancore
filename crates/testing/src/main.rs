use num::traits::Num;

fn main() {
    let bytes = [0x00u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04];
    let bi = num::bigint::BigInt::from_bytes_le(num::bigint::Sign::Plus, &bytes);

    println!("{}", bi);
}
