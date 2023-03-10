use nom::bytes::complete::take;
use nom::{Err, IResult, Needed};
use std::error::Error;

use crate::{LooseVarInt, VarInt, VarIntFindResult, VarIntInner, MSB};

macro_rules! ignore {
    ($($tt:tt)*) => {};
}

ignore! {
    fn varivari::VarInt::find_loose(slice: &[u8]) -> Option<LooseVarInt> {
        let (idx, _) = slice
            .iter()
            .enumerate()
            .take(VarInt::MAX_LEN)
            .find(|(_, &byte)| byte & MSB == 0)?;

        Some(LooseVarInt(&slice[..=idx]))
    }
}

pub fn varint_loose(input: &[u8]) -> IResult<&[u8], LooseVarInt, ()> {
    let Some((idx, _)) =
        input
            .iter()
            .enumerate()
            .take(VarInt::MAX_LEN)
            .find(|(_, &byte)| byte & MSB == 0)
        else {
            return Err(Err::Error(()))
        };

    Ok((&input[idx + 1..], LooseVarInt(&input[..=idx])))
}

pub fn varint(input: &[u8]) -> IResult<&[u8], VarInt, ()> {
    varint_loose(input).map(|(i, loose)| (i, loose.to_varint()))
}

ignore!(
    pub(crate) fn read_varint(input: &[u8]) -> IResult<&[u8], VarInt> {
        let mut len = 0;
        let mut buf: VarIntInner = [0; 5];

        // &[u8]
        for (idx, byte) in buf.iter_mut().enumerate() {

            // match self.read(slice::from_mut(byte)).await? {
            // match take(1u8)(&input) {
            //     Ok((input, value)) => {
            //         *byte = value;
            //         len = idx + 1;
            //         if *byte & MSB != MSB {
            //             break;
            //         }
            //     }
            //     Err(Err::Incomplete(Needed::Size(1))) => {
            //         return Err(Err::Incomplete(Needed::Size(1)));
            //     }
            //     Err(_) => {
            //         return Err(Err::Failure(Error::new(input, ErrorKind::Custom(0))));
            //     }
            // }
        }

        use VarIntFindResult::*;
        match len {
            0 => Err(Err::Failure(Error::new(input, ErrorKind::Custom(1)))),
            len => match VarInt::find_from_loose(LooseVarInt(&buf[..len])) {
                Tight(..) => Ok((
                    &input[buf.len()..],
                    VarInt {
                        inner: buf,
                        len: len as u8,
                    },
                )),
                Loose(_, actual_len) => {
                    buf[actual_len - 1] &= MSB;
                    buf[actual_len..len].fill(0);
                    Ok((
                        &input[buf.len()..],
                        VarInt {
                            inner: buf,
                            len: actual_len as u8,
                        },
                    ))
                }
                Invalid => Err(Err::Failure(Error::new(input, ErrorKind::Custom(2)))),
            },
        }
    }
);

ignore! {
    use nom::bytes::complete::take;
    use nom::error::{Error, ErrorKind, ParseError};
    use nom::{Err, IResult, Needed};

    use crate::{LooseVarInt, VarInt, VarIntFindResult, VarIntInner, MSB};

    pub fn read_varint<'a>(input: &[u8]) -> IResult<&'a [u8], usize, ()> {
        let mut len: usize = 0;
        let mut buf = [0; 5];

        for (idx, byte) in buf.iter_mut().enumerate() {
            match take(1)(input) {
                Ok((input, value)) => {
                    if *byte & MSB == 0 {
                        *byte = value;
                        len = idx + 1;
                        break;
                    }
                }
                Err(Err::Incomplete(Needed::Size(1))) => {
                    return Err(Err::Incomplete(Needed::Size(1)));
                }
            }
        }

        if len == 0 {
            return Err(Err::Failure(Error::new(input, ErrorKind::Fail)));
        }

        use VarIntFindResult::*;
        match VarInt::find_from_loose(LooseVarInt(&buf[..len])) {
            Tight(..) => {}
            Loose(_, actual_len) => {
                buf[actual_len - 1] &= MSB;
                buf[actual_len..len].fill(0);
                len = actual_len;
            }
            Invalid => {
                return Err(Err::Failure(Error::new(
                    input,
                    ErrorKind::InvalidData.into(),
                )))
            }
        }

        Ok(VarInt {
            inner: buf,
            len: len as u8,
        })
    }
}
