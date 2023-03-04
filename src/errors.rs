#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromVarIntSliceError(pub(crate) ());

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromVarIntInnerError(pub(crate) ());

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromLooseSliceError(pub(crate) ());

