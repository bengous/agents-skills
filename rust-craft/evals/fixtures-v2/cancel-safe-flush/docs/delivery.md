# Delivery contract

`BatchWriter::submit` queues records in order. `flush` processes that order.

- `Ok(())` means every record queued when flushing began was committed once.
- A sink error means the rejected record was not committed and remains first in
  the queue for an explicit retry.
- Cancelling a flush by dropping its future may discard the single in-flight
  record; it must not duplicate a committed record. Records not yet offered to
  the sink remain queued in order.
- The sink controls the commit point. A write future may commit a record and
  still return `Poll::Pending` before it reports completion.

The crate is dependency-free and supports Rust 1.85.
