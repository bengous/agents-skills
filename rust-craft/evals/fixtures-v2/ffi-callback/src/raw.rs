use std::ffi::c_void;

#[repr(C)]
pub(crate) struct RawParser {
    _private: [u8; 0],
}

pub(crate) type Callback = unsafe extern "C" fn(*mut c_void, *const u8, usize);

extern "C" {
    pub(crate) fn parser_new() -> *mut RawParser;
    pub(crate) fn parser_set_callback(
        parser: *mut RawParser,
        callback: Callback,
        context: *mut c_void,
    ) -> i32;
    pub(crate) fn parser_drain(parser: *mut RawParser) -> i32;
    pub(crate) fn parser_clear_callback(parser: *mut RawParser);
    pub(crate) fn parser_free(parser: *mut RawParser);
}
