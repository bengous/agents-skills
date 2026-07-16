# Async and Concurrency

## Execution Model

- An async expression creates a lazy `Future`; it is inert until actively polled, usually by an executor.
- Each `.await` is a potential suspension boundary. Dropping the future between polls can abandon it around that point, so shared and durable state must remain coherent.
- Async concurrency does not imply parallelism, though a multithreaded runtime may execute tasks in parallel. Blocking filesystem, CPU, lock, or foreign calls can still block an executor thread; use the runtime's established blocking boundary.
- Do not hand-roll `Future::poll`, pinning, or waker logic unless implementing the abstraction is the task.

## Task Lifecycle

- Name who owns each spawned task, how it stops, and who observes its result. Retain and await join handles when completion matters; detach only as an explicit contract.
- A spawned future commonly must own its captures and satisfy runtime-specific `'static` and `Send` bounds. Redesign ownership or task scope before adding blanket `Arc`, clones, or leaked data.
- Treat `select`, race, timeout, and early return as potential cancellation of unfinished in-scope futures. Runtime APIs differ for spawned tasks, which may keep running unless explicitly stopped; verify each losing operation's behavior.
- Design shutdown as a protocol: stop accepting work, signal cancellation, drain or reject queued work, await tasks, then release resources.
- Make backpressure explicit for producer/consumer systems. Unbounded queues trade immediate blocking for unbounded memory and shutdown work.

## `Send`, `Sync`, and Shared State

- `Send` means a value may move across threads; `Sync` means shared references may cross threads. They do not prove race-free higher-level behavior.
- A value held across `.await` becomes part of the future's suspended state and can make the future non-`Send`.
- Shorten scopes so guards and non-`Send` values are dropped before `.await` when no cross-await hold is required.
- Use a synchronous mutex for short non-awaiting critical sections when the runtime and contention permit it. Use an async mutex only when waiting must yield or a guard truly spans `.await`.
- Prefer ownership transfer through channels when it simplifies shared mutation, but choose channel capacity, closure, error, and ordering semantics deliberately.
- Never write an unsafe `Send` or `Sync` implementation without proving the underlying thread-safety contract, including foreign resources and callbacks.

## Validation

- Exercise stop during each meaningful await, task failure, receiver closure, queue saturation, and repeated shutdown where relevant.
- Make liveness tests bounded and deterministic. Avoid sleep-based confidence when barriers, notifications, paused time, or explicit handshakes exist in the chosen runtime.
- Compile the real spawn and trait-object paths; reasoning alone can miss inferred `Send`, lifetime, and object-safety constraints.

## Failure Modes

- Holding a standard mutex guard across `.await` and then forcing the future into a multithreaded spawn.
- Calling blocking work directly inside executor tasks without an established blocking boundary.
- Spawning a task and discarding its handle while claiming managed shutdown.
- Assuming cancellation rolls back partial I/O or state mutation.
- Replacing every shared value with `Arc<Mutex<_>>` to silence ownership errors.

## Official Sources

- [Async Book: async/await primer](https://rust-lang.github.io/async-book/01_getting_started/04_async_await_primer.html)
- [Async Book: the `Future` trait](https://rust-lang.github.io/async-book/02_execution/02_future.html)
- [Async Book: executors](https://rust-lang.github.io/async-book/02_execution/04_executor.html)
- [Async Book: `Send` approximation](https://rust-lang.github.io/async-book/07_workarounds/03_send_approximation.html)
- [The Rust Book: `Send` and `Sync`](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html)
- [`Sync` documentation](https://doc.rust-lang.org/std/marker/trait.Sync.html)
