use crate::{Hamming, MetricPoint};
use core::fmt::{Debug, Error, Formatter};
use core::hash::{Hash, Hasher};
#[cfg(feature = "serde")]
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

macro_rules! simd_impl {
    ($name:ident, $bytes:expr) => {
        #[repr(align($bytes))]
        #[derive(Copy, Clone)]
        pub struct $name(pub [u8; $bytes]);

        impl MetricPoint for Hamming<$name> {
            #[inline]
            fn distance(&self, rhs: &Self) -> u32 {
                // Perform an XOR popcnt. The compiler is smart
                // enough to optimize this well.
                (self.0)
                    .0
                    .iter()
                    .zip((rhs.0).0.iter())
                    .map(|(&a, &b)| (a ^ b).count_ones())
                    .sum()
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
                Debug::fmt(&self.0[..], f)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0[..] == other.0[..]
            }
        }

        impl Eq for $name {}

        impl Hash for $name {
            fn hash<H>(&self, state: &mut H)
            where
                H: Hasher,
            {
                self.0.hash(state)
            }
            fn hash_slice<H>(data: &[Self], state: &mut H)
            where
                H: Hasher,
            {
                for s in data {
                    s.hash(state);
                }
            }
        }

        impl Into<[u8; $bytes]> for $name {
            fn into(self) -> [u8; $bytes] {
                self.0
            }
        }

        impl From<[u8; $bytes]> for $name {
            fn from(a: [u8; $bytes]) -> Self {
                Self(a)
            }
        }

        #[cfg(feature = "serde")]
        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let a: [u8; $bytes] = self.clone().into();
                a.serialize(serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct SimdVisitor($name, usize);

                impl<'de> Visitor<'de> for SimdVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut Formatter) -> Result<(), Error> {
                        formatter.write_str("a sequence of $bytes bytes")
                    }

                    fn visit_seq<S>(mut self, mut seq: S) -> Result<$name, S::Error>
                    where
                        S: SeqAccess<'de>,
                    {
                        // Continuously fill the array with more values.
                        while let Some(value) = seq.next_element()? {
                            if self.1 == $bytes {
                                return Err(de::Error::custom(
                                    "cannot have more than $bytes bytes in sequence",
                                ));
                            }
                            (self.0).0[self.1] = value;
                            self.1 += 1;
                        }

                        if self.1 != $bytes {
                            Err(de::Error::custom(
                                "must have exactly $bytes bytes in sequence",
                            ))
                        } else {
                            Ok(self.0)
                        }
                    }
                }

                // Create the visitor and ask the deserializer to drive it. The
                // deserializer will call visitor.visit_seq() if a seq is present in
                // the input data.
                let visitor = SimdVisitor(Self([0; $bytes]), 0);
                deserializer.deserialize_seq(visitor)
            }
        }
    };
}

simd_impl!(Simd128, 16);
simd_impl!(Simd256, 32);
simd_impl!(Simd512, 64);
simd_impl!(Simd1024, 128);
simd_impl!(Simd2048, 256);
simd_impl!(Simd4096, 512);