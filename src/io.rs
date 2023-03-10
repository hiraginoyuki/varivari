#[cfg(feature = "std")]
pub use std_io::*;
#[cfg(feature = "std")]
mod std_io {
    use core::slice;
    use std::io::{self, Read, Write};

    use crate::{LooseVarInt, VarInt, VarIntFindResult::*, VarIntInner, MSB};

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
                        if *byte & MSB == 0 {
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
                    )),
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
                    len = actual_len;
                }
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
    use core::slice;

    use std::io;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    use crate::{LooseVarInt, VarInt, VarIntFindResult::*, VarIntInner, MSB};

    // r4: impl Read?
    #[async_trait]
    pub trait VarIntAsyncReadExt: AsyncRead {
        async fn read_varint(&mut self) -> io::Result<VarInt>
        where
            Self: Unpin,
        {
            let mut buf: VarIntInner = [0; 5];

            let mut len = 0;
            for (idx, byte) in buf.iter_mut().enumerate() {
                match self.read(slice::from_mut(byte)).await? {
                    // hot path 🥵
                    // breaks if continue bit is 0
                    1 => {
                        if *byte & MSB == 0 {
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
                    )),
                }
            }

            if len == 0 {
                return Err(io::ErrorKind::InvalidData.into());
            }

            match VarInt::find_from_loose(unsafe { LooseVarInt::from_unchecked(&buf[..len]) }) {
                Tight(..) => {}
                Loose(_, actual_len) => {
                    buf[actual_len - 1] &= MSB;
                    buf[actual_len..len].fill(0);
                    len = actual_len;
                }
            }

            Ok(VarInt {
                inner: buf,
                len: len as u8,
            })
        }
    }
    impl<R: AsyncRead> VarIntAsyncReadExt for R {}

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
