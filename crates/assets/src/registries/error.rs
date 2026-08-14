use core::{error::Error, fmt::Display};

use alloc::vec::Vec;

use crate::{BuildError, Name};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RegistriesError<S = Name> {
    DuplicateKind(&'static str),
    DuplicateKindStableId(S),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildError<S = Name> {
    pub(super) errors: Vec<RegistriesBuildErrorKind<S>>,
}

impl<S> RegistriesBuildError<S> {
    pub fn errors(&self) -> &[RegistriesBuildErrorKind<S>] {
        &self.errors
    }
}

impl<S: core::fmt::Display + core::fmt::Debug> Error for RegistriesBuildError<S> {}

impl<S: core::fmt::Display> Display for RegistriesBuildError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build this many kinds {}", self.errors.len())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegistriesBuildErrorKind<S = Name> {
    pub stable_id: S,
    pub build: BuildError<S>,
}

impl<S: core::fmt::Display + core::fmt::Debug + 'static> Error for RegistriesBuildErrorKind<S> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.build)
    }
}

impl<S: core::fmt::Display> Display for RegistriesError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RegistriesError::DuplicateKind(name) => write!(f, "{name} kind already exists"),
            RegistriesError::DuplicateKindStableId(stable_id) => {
                write!(
                    f,
                    "{stable_id} kind stable id already exists under different kind"
                )
            }
        }
    }
}

impl<S: core::fmt::Display> Display for RegistriesBuildErrorKind<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to build kind {}", self.stable_id)
    }
}

impl<S: core::fmt::Display + core::fmt::Debug> Error for RegistriesError<S> {}
