/*
Design Philosophy (kinda)

r1:     i32 -> VarInt
r2: [u8; 5] -> VarInt?
r3:   &[u8] -> VarInt?
r4: impl Read?
r5: impl AsyncRead?

w1: VarInt -> i32
w2: VarInt -> [u8; 5]
w3: VarInt ->&[u8; 5]
w4: VarInt ->&[u8]
w5: impl Write
w6: impl AsyncWrite
*/

//! MCMODERN's variable-length integers are fairly tricky to *properly* decode.
//!
//! varivari aims to provide the most ergonomic APIs to handle [`VarInt`]s by making sure that the following conversions are always possible.
//! ```
#![doc = concat!("# use ", module_path!(), "::{VarInt, VarIntInner};")]
//! # macro_rules! ascr {
//! #     ($expr:expr => $ty:ty) => {{
//! #         let tmp: $ty = $expr;
//! #         tmp
//! #     }}
//! # }
//! // Suppose we have all these:
//! const I32: i32 = 25565;
//! const BIN: i32 = 0b0000_0000000_0000001_1000111_1011101;
//! const ARR: [u8; 5] = [0b1101_1101, 0b1100_0111, 0b0000_0001, 0, 0];
//! assert_eq!(I32, BIN);
//!
//! // r1, w1: seamlessly convert between VarInt and i32
//! let foo = VarInt::from(I32);
//! let bar = i32::from(foo);
//! assert_eq!(I32, bar);
//!
//! // r2, r3, w2: extract or do a checked conversion from [u8; VarInt::MAX_LEN] (type-aliased as VarIntInner) or &[u8] to VarInt
//! let foo = VarInt::try_from(ARR.clone()).unwrap();
//! let bar = VarInt::try_from(&ARR[..3]).unwrap();
//! let qux = VarIntInner::from(foo.clone());
//! assert_eq!(BIN, i32::from(foo));
//! assert_eq!(BIN, i32::from(bar));
//! assert_eq!(ARR, qux);
//!
//! // w3, w4: AsRef<[u8]>, AsRef<VarIntInner>
//! let foo = VarInt::try_from(ARR.clone()).unwrap();
//! assert_eq!(&ARR, ascr!( foo.as_ref() => &[u8] ));
//! assert_eq!(&ARR[..3], ascr!( foo.as_ref() => &[u8] ));
//!
//! // r4, w5: VarIntReadExt: Read; VarIntWriteExt: Write;
//! // r5, w6: VarIntAsyncReadExt: AsyncRead; VarIntAsyncWriteExt: AsyncWrite;
//! ```

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::cmp;
use std::io::{self, Read, Write};

const MSB: u8 = 0b1000_0000;

pub enum VarIntFindResult<'a> {
    Tight(&'a [u8]),
    Loose(&'a [u8], usize),
    Invalid,
}

pub type VarIntInner = [u8; VarInt::MAX_LEN];
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct VarInt(VarIntInner);
impl VarInt {
    // ideal but div_ceil() is unstable atm
    // pub const MAX_LEN: usize = i32::BITS.div_ceil(7) as usize;
    pub const MAX_LEN: usize = 5;

    #[inline]
    fn find_loose(slice: &[u8]) -> Option<&[u8]> {
        let (idx, _) = slice
            .iter()
            .enumerate()
            .take(VarInt::MAX_LEN)
            .find(|(_, &byte)| byte & MSB == MSB)?;

        Some(&slice[..=idx])
    }

    #[inline]
    fn find(slice: &[u8]) -> VarIntFindResult {
        use VarIntFindResult::*;

        let Some(slice) = VarInt::find_loose(slice) else {
            return Invalid;
        };

        // SAFETY: `find_loose()` returns `None` and therefore `find()` returns `Invalid` at the above let-else, which makes it impossible for `silce` to be empty.
        if unsafe { slice.last().unwrap_unchecked() } & !MSB != 0 {
            return Tight(slice);
        }

        let Some((idx, _)) = slice
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &byte)| byte & !MSB != 0) else {
                return Invalid;
            };

        Loose(slice, idx + 1)
    }
}

// r1:     i32 -> VarInt
impl From<i32> for VarInt {
    fn from(source: i32) -> Self {
        let mut source = source as u32;
        let mut buf = [0u8; Self::MAX_LEN];

        for byte in buf.iter_mut() {
            *byte = source as u8 & !MSB;
            source >>= 7;
            if source == 0 {
                break;
            }
            *byte |= MSB
        }

        VarInt(buf)
    }
}

// r2: [u8; 5] -> VarInt?
impl TryFrom<VarIntInner> for VarInt {
    type Error = ();
    fn try_from(_: VarIntInner) -> Result<Self, Self::Error> {
        todo!()
    }
}

// r3:   &[u8] -> VarInt?
impl TryFrom<&[u8]> for VarInt {
    type Error = ();
    fn try_from(_: &[u8]) -> Result<Self, Self::Error> {
        todo!()
    }
}

// r4: impl Read?
pub trait VarIntReadExt: Read {
    fn read_varint(&mut self) -> io::Result<VarInt> {
        todo!()
    }
}
impl<R: Read> VarIntReadExt for R {}

// r5: impl AsyncRead?
#[async_trait]
pub trait VarIntAsyncReadExt: AsyncRead {
    async fn read_varint(&mut self) -> io::Result<VarInt>
    where
        Self: Unpin,
    {
        todo!()
    }
}
impl<R: AsyncRead> VarIntAsyncReadExt for R {}

// w1: VarInt -> i32
impl From<VarInt> for i32 {
    fn from(source: VarInt) -> Self {
        // source
        //     .0
        //     .into_iter()
        //     .enumerate()
        //     .fold(0u32, |acc, (idx, byte)| acc | (byte as u32) << (idx * 7)) as Self

        let mut result = 0u32;

        for (idx, byte) in source.0.into_iter().enumerate() {
            result |= (byte as u32) << (idx * 7);
        }

        result as Self
    }
}

// w2: VarInt -> [u8; 5]
impl From<VarInt> for VarIntInner {
    fn from(source: VarInt) -> Self {
        source.0
    }
}

// w3: VarInt ->&[u8; 5]
impl AsRef<VarIntInner> for VarInt {
    fn as_ref(&self) -> &VarIntInner {
        &self.0
    }
}

// w4: VarInt ->&[u8]
impl AsRef<[u8]> for VarInt {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

// w5: impl Write
pub trait VarIntWriteExt: Write {
    fn write_varint(&mut self, source: &VarInt) -> io::Result<()> {
        self.write_all(source.as_ref())
    }
}
impl<W: Write> VarIntWriteExt for W {}

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
