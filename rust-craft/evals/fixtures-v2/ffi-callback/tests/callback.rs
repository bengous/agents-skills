use ffi_callback::{Error, Parser};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

type Callback = unsafe extern "C" fn(*mut c_void, *const u8, usize);

struct MockParser {
    callback: Option<(Callback, *mut c_void)>,
}

static FREES: AtomicUsize = AtomicUsize::new(0);
static CLEARS: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
extern "C" fn parser_new() -> *mut c_void {
    Box::into_raw(Box::new(MockParser { callback: None })).cast()
}

#[no_mangle]
unsafe extern "C" fn parser_set_callback(
    parser: *mut c_void,
    callback: Callback,
    context: *mut c_void,
) -> i32 {
    // SAFETY: the wrapper passes the live handle returned by parser_new.
    let parser = unsafe { &mut *parser.cast::<MockParser>() };
    parser.callback = Some((callback, context));
    0
}

#[no_mangle]
unsafe extern "C" fn parser_drain(parser: *mut c_void) -> i32 {
    // SAFETY: the wrapper passes the live handle returned by parser_new.
    let parser = unsafe { &mut *parser.cast::<MockParser>() };
    if let Some((callback, context)) = parser.callback {
        let event = b"event";
        // SAFETY: event is valid for this call and context is the registered value.
        unsafe { callback(context, event.as_ptr(), event.len()) };
    }
    0
}

#[no_mangle]
unsafe extern "C" fn parser_clear_callback(parser: *mut c_void) {
    // SAFETY: the wrapper passes the live handle returned by parser_new.
    let parser = unsafe { &mut *parser.cast::<MockParser>() };
    parser.callback = None;
    CLEARS.fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
unsafe extern "C" fn parser_free(parser: *mut c_void) {
    // SAFETY: the wrapper transfers the allocation exactly once from Drop.
    let parser = unsafe { Box::from_raw(parser.cast::<MockParser>()) };
    assert!(
        parser.callback.is_none(),
        "callback must be cleared before free"
    );
    FREES.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn callback_lifetime_replacement_panic_and_cleanup_are_safe() {
    FREES.store(0, Ordering::SeqCst);
    CLEARS.store(0, Ordering::SeqCst);

    let first = Arc::new(Mutex::new(Vec::new()));
    let first_capture = Arc::clone(&first);
    let second = Arc::new(Mutex::new(Vec::new()));
    let second_capture = Arc::clone(&second);

    let mut parser = Parser::new().unwrap();
    parser
        .set_callback(move |event| first_capture.lock().unwrap().push(event.to_vec()))
        .unwrap();
    parser.drain().unwrap();
    assert_eq!(*first.lock().unwrap(), [b"event".to_vec()]);

    parser
        .set_callback(move |event| second_capture.lock().unwrap().push(event.to_vec()))
        .unwrap();
    parser.drain().unwrap();
    assert_eq!(*first.lock().unwrap(), [b"event".to_vec()]);
    assert_eq!(*second.lock().unwrap(), [b"event".to_vec()]);

    parser.set_callback(|_| panic!("callback panic")).unwrap();
    let drain = catch_unwind(AssertUnwindSafe(|| parser.drain()));
    assert_eq!(drain.unwrap(), Err(Error::CallbackPanicked));

    drop(parser);
    assert_eq!(CLEARS.load(Ordering::SeqCst), 1);
    assert_eq!(FREES.load(Ordering::SeqCst), 1);
}
