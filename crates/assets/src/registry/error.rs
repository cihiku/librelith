use core::error::Error;

use alloc::vec::Vec;

use crate::Name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError<S = Name> {
    DuplicateStableId(S),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError<S = Name> {
    pub(crate) errors: Vec<BuildErrorKind<S>>,
}

impl<S> BuildError<S> {
    pub fn errors(&self) -> &[BuildErrorKind<S>] {
        &self.errors
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildErrorKind<S = Name> {
    MissingComponent {
        entry: S,
        component: &'static str,
    },
    DuplicateComponent {
        entry: S,
        component: &'static str,
    },
    PinOutOfRange {
        entry: S,
        slot: u32,
        len: u32,
    },
    PinConflict {
        first: S,
        second: S,
        slot: u32,
    },
    DoublePin {
        entry: S,
        first: u32,
        second: u32,
    },
    DuplicateColumn {
        component: &'static str,
    },
    InsertOutOfRange {
        component: &'static str,
        id: u32,
        len: u32,
    },
    CapExceeded {
        len: u32,
        cap: u32,
    },
    DuplicateProperty {
        entry: S,
        property: &'static str,
    },
    TooManyStates {
        entry: S,
    },
    TooManyStatesTotal {
        entry: S,
    },
}

impl<S: core::fmt::Display> core::fmt::Display for RegistryError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DuplicateStableId(stable_id) => write!(f, "{stable_id} already in registry"),
        }
    }
}

impl<S: core::fmt::Display> core::fmt::Display for BuildError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let n = self.errors.len();
        write!(f, "{} build error{}:", n, if n == 1 { "" } else { "s" })?;
        for (i, error) in self.errors.iter().enumerate() {
            f.write_str(if i == 0 { " " } else { "; " })?;
            write!(f, "{error}")?
        }
        Ok(())
    }
}

impl<S: core::fmt::Display> core::fmt::Display for BuildErrorKind<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingComponent { entry, component } => {
                write!(f, "{entry} missing {component}")
            }
            Self::DuplicateComponent { entry, component } => {
                write!(f, "{entry} has duplicate {component}")
            }
            Self::PinOutOfRange { entry, slot, len } => {
                write!(f, "{entry} pinned to {slot} but registry has {len} entries")
            }
            Self::PinConflict {
                first,
                second,
                slot,
            } => {
                write!(f, "slot {slot} pinned to both {first} and {second}")
            }
            Self::DoublePin {
                entry,
                first,
                second,
            } => {
                write!(f, "{entry} pinned to both {first} and {second} slots")
            }
            Self::DuplicateColumn { component } => {
                write!(f, "{component} already exists")
            }
            Self::InsertOutOfRange { component, id, len } => {
                write!(
                    f,
                    "{component} value for id {id} but registry has {len} entries"
                )
            }
            Self::CapExceeded { len, cap } => {
                write!(f, "registry has {len} entries but cap is {cap}")
            }
            Self::DuplicateProperty { entry, property } => {
                write!(f, "{entry} has duplicate property {property}")
            }
            Self::TooManyStates { entry } => {
                write!(f, "{entry} state count overflows u32")
            }
            Self::TooManyStatesTotal { entry } => {
                write!(f, "total state count overflows u32 at {entry}")
            }
        }
    }
}

impl<S: core::fmt::Debug + core::fmt::Display> Error for RegistryError<S> {}
impl<S: core::fmt::Debug + core::fmt::Display> Error for BuildError<S> {}
