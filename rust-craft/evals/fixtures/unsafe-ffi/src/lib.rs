//! Deliberately incomplete safe wrapper exercise for a small parser C API.

use std::ffi::c_int;

#[repr(C)]
pub struct RawParser {
    _private: u8,
}

#[allow(dead_code)]
mod ffi {
    use super::{RawParser, c_int};

    unsafe extern "C" {
        pub fn parser_new() -> *mut RawParser;
        pub fn parser_parse(
            parser: *mut RawParser,
            input: *const u8,
            input_len: usize,
            out: *mut u8,
            out_cap: usize,
            written: *mut usize,
        ) -> c_int;
        pub fn parser_free(parser: *mut RawParser);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParserError {
    NullHandle,
    ForeignStatus(c_int),
    InvalidWritten { written: usize, capacity: usize },
}

pub struct Parser;

impl Parser {
    pub fn new() -> Result<Self, ParserError> {
        todo!("implement the safe construction boundary")
    }

    pub fn parse(&mut self, input: &[u8], output_capacity: usize) -> Result<Vec<u8>, ParserError> {
        let _ = (input, output_capacity);
        todo!("implement the safe parse boundary")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    };

    const SUCCESS: u8 = 0;
    const NULL_NEW: u8 = 1;
    const PARTIAL_WRITE_ERROR: u8 = 2;
    const OVERSIZED_WRITTEN: u8 = 3;

    static MODE: AtomicU8 = AtomicU8::new(SUCCESS);
    static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn configure(mode: u8) -> MutexGuard<'static, ()> {
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        MODE.store(mode, Ordering::Relaxed);
        FREE_CALLS.store(0, Ordering::Relaxed);
        guard
    }

    #[test]
    fn null_construction_is_an_error() {
        let _guard = configure(NULL_NEW);

        assert!(matches!(Parser::new(), Err(ParserError::NullHandle)));
    }

    #[test]
    fn successful_parse_returns_initialized_bytes() {
        let _guard = configure(SUCCESS);
        let mut parser = Parser::new().unwrap();

        assert_eq!(parser.parse(b"input", 3), Ok(b"yes".to_vec()));
    }

    #[test]
    fn partial_output_on_error_is_discarded() {
        let _guard = configure(PARTIAL_WRITE_ERROR);
        let mut parser = Parser::new().unwrap();

        assert_eq!(
            parser.parse(b"input", 3),
            Err(ParserError::ForeignStatus(17))
        );
    }

    #[test]
    fn reported_length_larger_than_capacity_is_rejected() {
        let _guard = configure(OVERSIZED_WRITTEN);
        let mut parser = Parser::new().unwrap();

        assert_eq!(
            parser.parse(b"input", 3),
            Err(ParserError::InvalidWritten {
                written: 4,
                capacity: 3,
            })
        );
    }

    #[test]
    fn parser_handle_is_freed_once() {
        let _guard = configure(SUCCESS);
        let parser = Parser::new().unwrap();

        drop(parser);
        assert_eq!(FREE_CALLS.load(Ordering::Relaxed), 1);
    }

    #[unsafe(no_mangle)]
    unsafe extern "C" fn parser_new() -> *mut RawParser {
        if MODE.load(Ordering::Relaxed) == NULL_NEW {
            std::ptr::null_mut()
        } else {
            Box::into_raw(Box::new(RawParser { _private: 0 }))
        }
    }

    #[unsafe(no_mangle)]
    unsafe extern "C" fn parser_parse(
        parser: *mut RawParser,
        input: *const u8,
        input_len: usize,
        out: *mut u8,
        out_cap: usize,
        written: *mut usize,
    ) -> c_int {
        assert!(!parser.is_null());
        assert!(!written.is_null());
        assert!(input_len == 0 || !input.is_null());
        assert!(out_cap == 0 || !out.is_null());

        match MODE.load(Ordering::Relaxed) {
            SUCCESS => {
                assert!(out_cap >= 3);
                // SAFETY: the foreign contract grants `out_cap` writable bytes.
                unsafe { std::ptr::copy_nonoverlapping(b"yes".as_ptr(), out, 3) };
                // SAFETY: checked non-null above.
                unsafe { written.write(3) };
                0
            }
            PARTIAL_WRITE_ERROR => {
                assert!(out_cap >= 2);
                // SAFETY: the foreign contract grants `out_cap` writable bytes.
                unsafe { std::ptr::copy_nonoverlapping(b"no".as_ptr(), out, 2) };
                // SAFETY: checked non-null above.
                unsafe { written.write(2) };
                17
            }
            OVERSIZED_WRITTEN => {
                // This intentionally does not write past `out_cap`; only the length lies.
                // SAFETY: checked non-null above.
                unsafe { written.write(out_cap + 1) };
                0
            }
            _ => unreachable!("test mode is configured by this module"),
        }
    }

    #[unsafe(no_mangle)]
    unsafe extern "C" fn parser_free(parser: *mut RawParser) {
        assert!(!parser.is_null());
        FREE_CALLS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: this test symbol receives each allocation from `parser_new` once.
        unsafe { drop(Box::from_raw(parser)) };
    }
}
