/* Foreign callback contract used by this fixture.

   parser_set_callback stores both callback and context until replacement or
   parser_clear_callback. On failure it leaves the previous registration
   unchanged. A successful registration is invoked only by parser_drain on the
   same thread. The data pointer is valid only for the duration of that call.

   A callback must not unwind across this C ABI. The parser is not thread-safe.
   parser_clear_callback must run before callback context is destroyed.
   parser_free accepts one live parser and must be called exactly once.
*/
