#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::Write;
use defmt::error;
use heapless::String;
pub type Result<T> = core::result::Result<T, &'static str>;

#[extension_traits::extension(pub trait IntoWrapErrDebugExt)]
impl<T, E: core::fmt::Debug> core::result::Result<T, E> {
    fn into_wrap_err_dbg(self, context: &'static str) -> Result<T> {
        self.map_err(|e| {
            let mut out = String::<256>::new();
            write!(&mut out, "{:?}", e).ok();
            error!("ERROR: {:?}", out);
            context
        })
    }
}
#[extension_traits::extension(pub trait IntoWrapErrExt)]
impl<T, E: defmt::Format> core::result::Result<T, E> {
    fn into_wrap_err(self, context: &'static str) -> Result<T> {
        self.map_err(|e| {
            error!("ERROR: {:?}", e);
            context
        })
    }
}

#[extension_traits::extension(pub trait WrapErrorExt)]
impl<T> Result<T> {
    fn wrap_err(self, context: &'static str) -> Result<T> {
        self.map_err(|error| {
            error!("CAUSED BY: {}", error);
            context
        })
    }
}
