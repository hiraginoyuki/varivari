/*
VarIntが正しくないケース一覧
A: MSBが1のバイトが4つ以下あるが、その後ろにバイトが来ない (EOF | From<&[u8]>の場合, 回復不能)
B: 5個目まで全てのバイトのMSBが1 (回復不能)
C: 5個目のバイトの常に0の部分(-000 ---- の0)が1 (回復可能, 0埋め)
D: 正しいVarIntシーケンスの後ろにまだバイトがある (回復可能, From<&[u8]>のみ?, 無視)

Q1: AとBは区別されるべき?
A1: Yes. AはTooLong的な側面があるのに対して, Bはそうではなく, バイトが足りないから起きるものであるので、区別されるべき。
*/

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

//! ```
//! # macro_rules! _{( // bypass doctest for now
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
//! assert_eq!(&ARR, foo.as_ref());
//! assert_eq!(&ARR[..3], foo.as_ref());
//!
//! // r4, w5: VarIntReadExt: Read; VarIntWriteExt: Write;
//! // r5, w6: VarIntAsyncReadExt: AsyncRead; VarIntAsyncWriteExt: AsyncWrite;
//! # )=>{}}
//! ```

//! MCMODERN's variable-length integers are fairly tricky to *properly* decode.

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::io::{self, Read, Write};

const MSB: u8 = 0b1000_0000;

pub enum VarIntFindResult<'a> {
    Tight(&'a [u8]),
    Loose(&'a [u8], usize),
    TooFew,
    TooMany,
}

pub type VarIntInner = [u8; VarInt::MAX_LEN];
#[repr(transparent)]
pub struct VarInt(VarIntInner);
impl VarInt {
    // ideal but div_ceil() is unstable atm
    // pub const MAX_LEN: usize = i32::BITS.div_ceil(7) as usize;
    pub const MAX_LEN: usize = 5;

    #[inline]
    fn find(slice: &[u8], size_hint: Option<usize>) -> VarIntFindResult {
        use VarIntFindResult::*;

        todo!()
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
