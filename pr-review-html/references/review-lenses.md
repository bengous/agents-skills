# Review Lenses

Load this file when the user names a difficult review domain, especially streaming, backpressure, cancellation, retries, async iteration, or queueing.

## Streaming And Backpressure

Use this checklist to find real review risks. Anchor any finding to changed lines or missing tests.

- Flow-control contract: identify the producer, buffer, consumer, and who is allowed to slow whom down.
- Boundedness: check for unbounded arrays, queues, pending promises, unread chunks, retained buffers, or retry loops.
- Backpressure signal: verify that `write()`/enqueue/send return values, `drain`, readiness promises, permits, credits, or demand counters are respected instead of ignored.
- Await points: confirm that async writes, flushes, reads, and cancellation paths are awaited where ordering or memory pressure depends on them.
- Cancellation: check `AbortSignal`, stream `cancel`, iterator `return`, connection close, task abort, and subscription cleanup paths.
- Error propagation: ensure producer and consumer failures terminate the stream or surface to the caller; avoid logging and continuing with corrupted state.
- Terminal events: verify end-of-stream, final flush, half-close, and double-close behavior.
- Ordering: check whether concurrent writes, retries, batching, or transforms can reorder chunks unexpectedly.
- Resource cleanup: confirm timers, listeners, file descriptors, sockets, subscriptions, and spawned tasks are released on success, error, and cancellation.
- Retry and reconnect: verify idempotency, replay boundaries, duplicate delivery, and whether buffered chunks survive reconnects safely.
- Tests: look for slow-consumer, fast-producer, cancellation-mid-stream, error-mid-stream, partial-write, retry, and cleanup assertions.

## Artifact Explanation Pattern

When the logic is subtle, include a small visual model in the HTML artifact:

- "Normal path" showing producer to buffer to consumer.
- "Pressure path" showing what should happen when the consumer slows down.
- "Failure path" showing cancellation or error propagation.
- "Changed lines" linking the model back to diff anchors.

Keep the model concrete. Prefer names from the code over generic labels.
