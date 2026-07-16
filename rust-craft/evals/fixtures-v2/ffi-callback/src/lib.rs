mod raw;

use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NullHandle,
    ForeignStatus(i32),
    CallbackPanicked,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Error {}

/// Owning parser handle. The foreign parser is thread-affine.
///
/// ```compile_fail
/// use ffi_callback::Parser;
/// fn assert_send<T: Send>() {}
/// assert_send::<Parser>();
/// ```
pub struct Parser {
    raw: NonNull<raw::RawParser>,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl Parser {
    pub fn new() -> Result<Self, Error> {
        // SAFETY: parser_new takes no arguments and returns an owned handle or null.
        let raw = unsafe { raw::parser_new() };
        let raw = NonNull::new(raw).ok_or(Error::NullHandle)?;

        Ok(Self {
            raw,
            _not_send_sync: PhantomData,
        })
    }

    pub fn set_callback<F>(&mut self, callback: F) -> Result<(), Error>
    where
        F: FnMut(&[u8]) + 'static,
    {
        let _ = callback;
        todo!("store callback context for the foreign registration")
    }

    pub fn drain(&mut self) -> Result<(), Error> {
        todo!("drain callbacks without unwinding across the C ABI")
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        // SAFETY: self owns this live handle and Drop runs once.
        unsafe { raw::parser_free(self.raw.as_ptr()) }
    }
}
