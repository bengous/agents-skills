# Managed async shutdown

Make `./check.sh` pass without adding dependencies.

`Service::start` begins one managed operation and returns the task that owns
that operation. `request_stop` asks it to finish. Once the caller joins that
task after requesting stop, the service must report `Phase::Stopped` and one
completed operation.

Keep the public API used by `tests/lifecycle.rs`. `Runtime::spawn` deliberately
models the bounds imposed by a multithreaded executor. Do not weaken those
bounds, detach the operation, use `unsafe`, or add dependencies.
