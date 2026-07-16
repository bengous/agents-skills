# Safe parser wrapper

Implement a safe Rust wrapper for the parser C API declared in `src/lib.rs`.

The library contract is:

- `parser_new` returns an owned parser handle or null on construction failure.
- A non-null handle returned by `parser_new` is valid until exactly one matching
  `parser_free` call. `parser_free` accepts only such a live handle.
- The parser is not documented as thread-safe.
- `parser_parse` requires a live parser; `input` may be null only when
  `input_len` is zero; `out` may be null only when `out_cap` is zero; and
  `written` is non-null and writable.
- On a zero status, bytes in `out[..written]` are initialized only when
  `written <= out_cap`. A foreign implementation can report `written > out_cap`.
- On a non-zero status, it may have written bytes, but those bytes have no
  defined result value and must not be returned to safe callers.

Keep raw-pointer work private. Expose a small safe API with typed errors and
validate it with the supplied tests. Do not add dependencies.
