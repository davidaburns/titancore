#[macro_export]
macro_rules! define_key_sized {
    ($name: ident, $size: expr) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $name {
            key: [u8; $size],
        }

        #[allow(dead_code)]
        impl $name {
            pub const SIZE: usize = $size;

            pub fn from_bytes_le(bytes: &[u8; $size]) -> Self {
                Self { key: bytes.clone() }
            }

            pub fn from_hex_str(hex_str: &str) -> Result<Self, hex::FromHexError> {
                let mut bytes = hex::decode(hex_str)?;
                while bytes.len() < $size {
                    bytes.insert(0, 0x00);
                }

                let key = <[u8; $size]>::try_from(bytes).unwrap();
                Ok(Self { key })
            }

            pub fn from_hex_str_be(hex_str: &str) -> Result<Self, hex::FromHexError> {
                let mut bytes = hex::decode(hex_str)?;
                while bytes.len() < $size {
                    bytes.insert(0, 0x00);
                }

                bytes.reverse();

                let key = <[u8; $size]>::try_from(bytes).unwrap();
                Ok(Self { key })
            }

            pub fn as_bytes_le(&self) -> [u8; $size] {
                self.key
            }

            pub fn as_bytes_be(&self) -> [u8; $size] {
                let mut key_clone = self.key.clone();
                key_clone.reverse();
                key_clone
            }

            pub fn as_split_slice(&self) -> &[u8] {
                let mut s = &self.key[..];
                let mut lead = 0;
                while s[lead] == 0 {
                    lead += 1;
                }

                if lead % 2 != 0 {
                    lead += 1;
                }

                s = &s[lead..];
                s
            }

            pub fn to_vec(&self) -> Vec<u8> {
                let mut s = self.to_bigint().to_bytes_le().1;
                if s[0] == 0 {
                    s = s[1..].to_vec();
                }
                if s.len() % 2 != 0 {
                    s = s[1..].to_vec();
                }

                s
            }

            pub fn to_hex_str(&self) -> String {
                hex::encode(self.key)
            }

            pub fn to_hex_str_be(&self) -> String {
                let mut key_clone = self.key.clone();
                key_clone.reverse();

                hex::encode(key_clone)
            }

            pub fn to_bigint(&self) -> num::bigint::BigInt {
                num::bigint::BigInt::from_bytes_le(num::bigint::Sign::Plus, &self.key)
            }
        }

        impl From<num::bigint::BigInt> for $name {
            fn from(b: num::bigint::BigInt) -> Self {
                let mut key = [0u8; $size];
                let b = b.to_bytes_le().1.to_vec();
                key[0..b.len()].clone_from_slice(&b);

                Self { key }
            }
        }

        impl Into<num::bigint::BigInt> for $name {
            fn into(self) -> num::bigint::BigInt {
                self.to_bigint()
            }
        }

        impl From<[u8; $size]> for $name {
            fn from(b: [u8; $size]) -> Self {
                Self { key: b }
            }
        }

        impl Eq for $name {}
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                let other = other.as_bytes_le();
                for (i, value) in self.key.iter().enumerate() {
                    if *value != other[i] {
                        return false;
                    }
                }

                true
            }
        }
    };
}

#[macro_export]
macro_rules! define_key_constant {
    ($name: ident, $size: expr, $value: expr) => {
        define_key_sized!($name, $size);

        impl Default for $name {
            fn default() -> Self {
                Self { key: $value }
            }
        }
    };
}

#[macro_export]
macro_rules! define_byte_value {
    ($name: ident, $value: expr) => {
        #[derive(Debug)]
        pub struct $name(u8);

        impl Default for $name {
            fn default() -> Self {
                Self { 0: $value }
            }
        }

        #[allow(dead_code)]
        impl $name {
            pub fn from_value(value: u8) -> Self {
                Self { 0: value }
            }

            pub fn value(&self) -> u8 {
                self.0
            }

            pub fn to_bigint(&self) -> num::bigint::BigInt {
                num::bigint::BigInt::from(self.0)
            }
        }
    };
}

#[cfg(test)]
mod test {
    use crate::{define_byte_value, define_key_constant, define_key_sized};

    const TEST_BYTE_VALUE: u8 = 10;
    const TEST_KEY_SIZED_SIZE: usize = 10;
    const TEST_KEY_SIZED_HEX_STR: &str = "000000000000DEADBEEF";
    const TEST_KEY_SIZED_HEX_BYTES: [u8; TEST_KEY_SIZED_SIZE] =
        [0x00u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

    define_byte_value!(TestByteValue, TEST_BYTE_VALUE);
    define_key_sized!(TestKeySized, TEST_KEY_SIZED_SIZE);
    define_key_constant!(
        TestKeyConstant,
        TEST_KEY_SIZED_SIZE,
        TEST_KEY_SIZED_HEX_BYTES
    );

    #[test]
    fn test_define_byte_value_proper_value() {
        assert_eq!(TEST_BYTE_VALUE, TestByteValue::default().value());
    }

    #[test]
    fn test_define_byte_value_proper_bigint() {
        let expected = num::bigint::BigInt::from(TEST_BYTE_VALUE);
        assert_eq!(expected, TestByteValue::default().to_bigint());
    }

    #[test]
    fn test_define_key_sized_size() {
        assert_eq!(TEST_KEY_SIZED_SIZE, TestKeySized::SIZE);
    }

    #[test]
    fn test_define_key_sized_from_bytes_le() {
        let expected = TestKeySized {
            key: TEST_KEY_SIZED_HEX_BYTES,
        };

        let a = TestKeySized::from_bytes_le(&TEST_KEY_SIZED_HEX_BYTES);
        assert_eq!(expected, a);
    }

    #[test]
    fn test_define_key_sized_from_hex_str() {
        let expected = TestKeySized {
            key: TEST_KEY_SIZED_HEX_BYTES,
        };

        let a = TestKeySized::from_hex_str(TEST_KEY_SIZED_HEX_STR).unwrap();
        assert_eq!(expected, a);
    }

    #[test]
    fn test_define_key_sized_from_hex_str_pads_bytes() {
        let expected = TestKeySized {
            key: TEST_KEY_SIZED_HEX_BYTES,
        };

        let a = TestKeySized::from_hex_str("DEADBEEF").unwrap();
        assert_eq!(expected, a);
    }

    #[test]
    fn test_define_key_sized_from_hex_str_errors() {
        let a = TestKeySized::from_hex_str("ISNOTHEX");
        assert!(a.is_err())
    }

    #[test]
    fn test_define_key_sized_to_bigint_from_bytes() {
        let a = TestKeySized::from_bytes_le(&TEST_KEY_SIZED_HEX_BYTES);
        let expected: u128 = 1132162999231063412178944;

        assert_eq!(expected.to_string(), a.to_bigint().to_string());
    }

    #[test]
    fn test_define_key_sized_to_bigint_from_hex_str() {
        let a = TestKeySized::from_hex_str(TEST_KEY_SIZED_HEX_STR).unwrap();
        let expected: u128 = 1132162999231063412178944;

        assert_eq!(expected.to_string(), a.to_bigint().to_string());
    }
}
