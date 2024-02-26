#![cfg_attr(not(feature = "std"), no_std)]

use log::error;
pub type Result<T> = core::result::Result<T, &'static str>;

#[extension_traits::extension(pub trait IntoWrapErrExt)]
impl<T, E: core::fmt::Debug> core::result::Result<T, E> {
    fn into_wrap_err(self, context: &'static str) -> Result<T> {
        self.map_err(|e| {
            error!("ERROR: {e:?}");
            context
        })
    }
}

#[extension_traits::extension(pub trait WrapErrorExt)]
impl<T> Result<T> {
    fn wrap_err(self, context: &'static str) -> Result<T> {
        self.map_err(|error| {
            error!("CAUSED BY: {error}");
            context
        })
    }
}
