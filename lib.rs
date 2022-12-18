//! varivari aims to provide the most ergonomic APIs to handle [`VarInt`]s by making sure that the following conversions are always possible.
//! ```
#![doc = concat!("# use ", module_path!(), "::{VarInt, VarIntInner};")]
//! # macro_rules! ascribe {
//! #     ($expr:expr => $ty:ty) => {{
//! #         let tmp: $ty = $expr;
//! #         tmp
//! #     }}
//! # }
//! // The same value in different representations
//! const I32: i32 = 25565;
//! const U32: u32 = 25565;
//! const BIN: i32 = 0b0000_0000000_0000001_1000111_1011101;
//! const ARR: [u8; 5] = [0b1101_1101, 0b1100_0111, 0b0000_0001, 0, 0];
//! let VARINT: VarInt = ARR.clone().try_into().unwrap();
//! assert_eq!(I32, BIN);
//! assert_eq!(&ARR, VARINT.as_inner());
//!
//! { // between VarInt, i32 and u32
//!     let foo = VarInt::from(I32);
//!     let bar = i32::from(foo.clone());
//!     let qux = u32::from(foo.clone());
//!     assert_eq!(I32, bar);
//!     assert_eq!(U32, qux);
//! }
//!
//! { // between array and silce, and VarInt
//!     let foo = VarInt::try_from(ARR.clone()).unwrap();
//!     let bar = VarInt::try_from(&ARR[..3]).unwrap();
//!     let qux = VarIntInner::from(foo.clone());
//!     assert_eq!(BIN, i32::from(foo));
//!     assert_eq!(BIN, i32::from(bar));
//!     assert_eq!(ARR, qux);
//! }
//!
//! { // AsRef<[u8]>, AsRef<VarIntInner>
//!     let foo = VarInt::try_from(ARR.clone()).unwrap();
//!     assert_eq!(&ARR,      foo.as_inner());
//!     assert_eq!(&ARR[..3], foo.as_slice());
//!     assert_eq!(&ARR,      ascribe!( foo.as_ref() => &VarIntInner ));
//!     assert_eq!(&ARR[..3], ascribe!( foo.as_ref() => &[u8] ));
//! }
//!
//! { // trait VarIntReadExt: Read;
//!   // trait VarIntWriteExt: Write;
//!     use std::io::{Read, Write};
#![doc = concat!("    use ", module_path!(), "::{VarIntReadExt, VarIntWriteExt};")]
//!
//!     // `&[u8]` implements `Read`
//!     let foo = ARR.as_ref().read_varint().unwrap();
//!     assert_eq!(ARR, foo.into_inner());
//!
//!     // `&mut [u8]` implements `Write`
//!     let mut bar: VarIntInner = [0; 5];
//!     bar.as_mut_slice().write_varint(&VARINT);
//!     assert_eq!(ARR, bar);
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromVarIntSliceError(pub(crate) ());

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromVarIntInnerError(pub(crate) ());

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromLooseSliceError(pub(crate) ());

pub(crate) const MSB: u8 = 0b1000_0000;

use core::hint::unreachable_unchecked;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VarIntFindResult<'a> {
    Tight(&'a [u8]),
    Loose(&'a [u8], usize),
    Invalid,
}

#[derive(Debug, Copy, Clone)]
pub struct LooseVarInt<'a>(pub(crate) &'a [u8]);
impl<'a> LooseVarInt<'a> {
    pub fn into_inner(self) -> &'a [u8] {
        self.0
    }
    pub fn as_inner(&'a self) -> &[u8] {
        self.0
    }
}
impl<'a> AsRef<[u8]> for LooseVarInt<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

pub type VarIntInner = [u8; VarInt::MAX_LEN];
#[derive(Debug, Clone)]
pub struct VarInt {
    inner: VarIntInner,
    len: u8,
}

impl VarInt {
    // ideal but div_ceil() is unstable atm
    // pub const MAX_LEN: usize = i32::BITS.div_ceil(7) as usize;
    pub const MAX_LEN: usize = 5;
    pub const LAST_BYTE_MASK: u8 = 0b0000_1111;
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.inner[..self.len() as usize]
    }
    pub const fn as_inner(&self) -> &VarIntInner {
        &self.inner
    }
    pub const fn into_inner(self) -> VarIntInner {
        self.inner
    }

    #[inline]
    fn find_loose(slice: &[u8]) -> Option<LooseVarInt> {
        let (idx, _) = slice
            .iter()
            .enumerate()
            .take(VarInt::MAX_LEN)
            .find(|(_, &byte)| byte & MSB == 0)?;

        Some(LooseVarInt(&slice[..=idx]))
    }

    #[inline]
    fn find_from_loose<'a>(loose: LooseVarInt<'a>) -> VarIntFindResult<'a> {
        use VarIntFindResult::*;

        let slice = loose.as_ref();

        // SAFETY: `find_loose()` returns `None` and therefore `find()` returns
        // `Invalid` at the above let-else, which makes it impossible for
        // `silce` to be empty.
        if slice.len() == 1 || unsafe { slice.last().unwrap_unchecked() } & !MSB != 0 {
            return Tight(loose.0);
        }

        let Some((idx, _)) = slice
            .iter()
            .enumerate()
            .rev()
            .skip(1)
            .find(|(_, &byte)| byte & !MSB != 0) else {
                return Invalid;
            };

        Loose(loose.0, idx + 1)
    }

    #[inline]
    fn find(slice: &[u8]) -> VarIntFindResult {
        let Some(slice) = VarInt::find_loose(slice) else {
            return VarIntFindResult::Invalid;
        };

        VarInt::find_from_loose(slice)
    }
}

// r1: i32 -> VarInt
impl From<u32> for VarInt {
    fn from(mut source: u32) -> Self {
        let mut buf = [0u8; Self::MAX_LEN];

        for (idx, byte) in buf.iter_mut().enumerate() {
            *byte = source as u8 & !MSB;
            source >>= 7;
            if source == 0 {
                return VarInt {
                    inner: buf,
                    len: idx as u8 + 1,
                };
            }
            *byte |= MSB
        }

        // SAFETY: `buf` always has 5 elements and the loop always breaks,
        // because at 5th iteration, `source == 0` is the same as
        // `(whatever_u32 >> 35) == 0` which is always true.
        unsafe { unreachable_unchecked() }
    }
}
impl From<i32> for VarInt {
    fn from(source: i32) -> Self {
        (source as u32).into()
    }
}

// r2: [u8; 5] -> VarInt?
impl TryFrom<VarIntInner> for VarInt {
    type Error = TryFromVarIntInnerError;
    fn try_from(mut source: VarIntInner) -> Result<Self, Self::Error> {
        use VarIntFindResult::*;

        let len = match VarInt::find(&source) {
            Invalid => return Err(TryFromVarIntInnerError(())),
            Tight(slice) => slice.len(),
            Loose(_, actual_len) => {
                source[actual_len - 1] &= !MSB;
                actual_len
            }
        };

        source[len..].fill(0);

        Ok(VarInt {
            inner: source,
            len: len as u8,
        })
    }
}

// r3: &[u8] -> VarInt?
impl TryFrom<&[u8]> for VarInt {
    type Error = TryFromVarIntSliceError;
    fn try_from(source: &[u8]) -> Result<Self, Self::Error> {
        use VarIntFindResult::*;

        let mut buf: VarIntInner = [0; 5];

        let len = match VarInt::find(&source) {
            Invalid => return Err(TryFromVarIntSliceError(())),
            Tight(slice) => slice.len(),
            Loose(_, actual_len) => {
                buf[actual_len - 1] &= !MSB;
                actual_len
            }
        };

        buf[..len].copy_from_slice(&source[..len]);

        Ok(VarInt {
            inner: buf,
            len: len as u8,
        })
    }
}

// impl TryFrom<LooseVarInt<'_>> for VarInt {
//     type Error = TryFromLooseSliceError;
//     fn try_from(value: LooseVarInt) -> Result<Self, Self::Error> {
//         use VarIntFindResult::*;

//         match VarInt::find_from_loose(value) {
//             Tight(slice) =>
//         }
//     }
// }

// w1: VarInt -> i32
impl From<VarInt> for u32 {
    #[inline]
    fn from(source: VarInt) -> Self {
        let mut result = 0;

        for (idx, byte) in source.inner.into_iter().enumerate() {
            result |= (byte as u32) << (idx * 7);
        }

        result
    }
}

impl From<VarInt> for i32 {
    #[inline]
    fn from(source: VarInt) -> Self {
        u32::from(source) as Self
    }
}

// w2: VarInt -> [u8; 5]
impl From<VarInt> for VarIntInner {
    fn from(source: VarInt) -> Self {
        source.inner
    }
}

// w3: VarInt ->&[u8; 5]
impl AsRef<VarIntInner> for VarInt {
    fn as_ref(&self) -> &VarIntInner {
        &self.inner
    }
}

// w4: VarInt ->&[u8]
impl AsRef<[u8]> for VarInt {
    fn as_ref(&self) -> &[u8] {
        &self.inner[..self.len as usize]
    }
}

#[cfg(feature = "std")]
pub use std_io::*;
#[cfg(feature = "std")]
mod std_io {
    use core::slice;
    use std::io::{self, Read, Write};

    use super::{LooseVarInt, VarInt, VarIntFindResult::*, VarIntInner, MSB};

    // r4: impl Read?
    pub trait VarIntReadExt: Read {
        fn read_varint(&mut self) -> io::Result<VarInt> {
            let mut buf: VarIntInner = [0; 5];

            let mut len = 0;
            for (idx, byte) in buf.iter_mut().enumerate() {
                match self.read(slice::from_mut(byte))? {
                    // hot path 🥵
                    // breaks if continue bit is 0
                    1 => {
                        if *byte & MSB != MSB {
                            len = idx + 1;
                            break;
                        }
                    }

                    // really cold path 🥶
                    // handles unexpected EOF
                    0 => return Err(io::ErrorKind::UnexpectedEof.into()),

                    // super duper cold path 🧊
                    // SAFETY: Read states that n <= buf.len() is not guaranteed,
                    // so unreachable_unchecked cannot be used here.
                    _ => unreachable!(concat!(
                        "This is a bug of ",
                        env!("CARGO_PKG_REPOSITORY"),
                        ". Please create an issue to report it."
                    ))
                }
            }

            if len == 0 {
                return Err(io::ErrorKind::InvalidData.into());
            }

            match VarInt::find_from_loose(LooseVarInt(&buf[..len])) {
                Tight(..) => {}
                Loose(_, actual_len) => {
                    buf[actual_len - 1] &= MSB;
                    buf[actual_len..len].fill(0);
                }
                Invalid => return Err(io::ErrorKind::InvalidData.into()),
            }

            Ok(VarInt {
                inner: buf,
                len: len as u8,
            })
        }
    }
    impl<R: Read> VarIntReadExt for R {}

    // w5: impl Write
    pub trait VarIntWriteExt: Write {
        fn write_varint(&mut self, source: &VarInt) -> io::Result<()> {
            self.write_all(source.as_ref())
        }
    }
    impl<W: Write> VarIntWriteExt for W {}
}

#[cfg(feature = "tokio")]
pub use tokio_io::*;
#[cfg(feature = "tokio")]
mod tokio_io {
    use async_trait::async_trait;

    use std::io;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    use super::VarInt;

    // w6: impl AsyncWrite
    #[async_trait]
    pub trait VarIntAsyncWriteExt: AsyncWrite {
        async fn write_varint(&mut self, source: &VarInt) -> io::Result<()>
        where
            Self: Unpin,
        {
            self.write_all(source.as_ref()).await
        }
    }
    impl<W: AsyncWrite> VarIntAsyncWriteExt for W {}
}
