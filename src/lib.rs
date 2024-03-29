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

macro_rules! ignore {
    ($($tt:tt)*) => {};
}


mod errors;
pub mod io;
pub mod nom;

pub use errors::*;

pub(crate) const MSB: u8 = 0b1000_0000;

use core::hint::unreachable_unchecked;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VarIntFindResult<'a> {
    Tight(&'a [u8]),
    Loose(&'a [u8], usize),
}

#[derive(Debug, Copy, Clone)]
pub struct LooseVarInt<'a>(&'a [u8]);
impl LooseVarInt<'_> {
    pub const fn inner(&self) -> &[u8] {
        self.0
    }

    /// # Safety
    /// `slice.len()` must be contained in `1..=VarInt::MAX_LEN`.
    pub unsafe fn from_unchecked(slice: &[u8]) -> LooseVarInt<'_> {
        LooseVarInt(slice)
    }

    ignore! {
        Tight(0_000_0000) = 0             // len == 1, looks loose but actually tight
        Loose(1_000_0000, 0_000_0000) = 0 // obviously loose
        Loose(1_000_0001, 0_000_0000) = 0b1_0000000
        Tight(1_000_0001, 0_000_0001) = 0b1_0000001
        Loose(1_000_0001, 1_000_0001, 0_000_0000) = 0b1_0000001_0000000
        Tight(1_000_0001, 1_000_0001, 0_000_0001) = 0b1_0000001_0000001
    }

    pub fn to_varint(&self) -> VarInt {
        let slice = self.0;
        let inner: VarIntInner = [0; 5];

        debug_assert!(slice.len() < VarInt::MAX_LEN);

        if slice.len() == 1 || unsafe { slice.last().unwrap_unchecked() } & !MSB != 0 {
            return VarInt { inner, len: slice.len() as u8 }
        }

        let len = slice
            .iter()
            .enumerate()
            .rev()
            .skip(1) // because it's checked above not to be tight
            .find(|(_, &byte)| byte & !MSB != 0)
            .map(|(idx, _)| idx + 1)
            .unwrap_or(1); // not unwrap_unchecked because it might be 0 of any (0..=5) length

        todo!()
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
        let (idx, _) = slice.iter().enumerate()
            .take(VarInt::MAX_LEN) // None if MSB is in slice[VarInt::MAX_LEN..]
            .find(|(_, &byte)| byte & MSB == 0)?; // None if MSB of the slice is [1, 1, 1, 1, 1]

        Some(unsafe {
            // SAFETY: `Iterator::take` and returning (by `?`) if the result is None ensures that `idx + 1` is contained in 1..=VarInt::MAX_LEN
            LooseVarInt::from_unchecked(&slice[..=idx])
        })
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

        let len = slice
            .iter()
            .enumerate()
            .rev()
            .skip(1) // because it's checked above not to be tight
            .find(|(_, &byte)| byte & !MSB != 0)
            .map(|(idx, _)| idx + 1)
            .unwrap_or(1); // not unwrap_unchecked because it might be 0 of any (1..=VarInt::MAX_LEN) length

        Loose(loose.0, len)
    }

    #[inline]
    fn find(slice: &[u8]) -> Option<VarIntFindResult> {
        VarInt::find_loose(slice).map(VarInt::find_from_loose)
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
            None => return Err(TryFromVarIntInnerError(())),
            Some(Tight(slice)) => slice.len(),
            Some(Loose(_, actual_len)) => {
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
            None => return Err(TryFromVarIntSliceError(())),
            Some(Tight(slice)) => slice.len(),
            Some(Loose(_, actual_len)) => {
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
