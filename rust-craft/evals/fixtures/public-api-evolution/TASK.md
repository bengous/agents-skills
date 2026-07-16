# Task

The next minor release must let stores flush buffered writes, expose basic operation metrics, and offer an asynchronous write path.

Evolve the public API without breaking existing downstream `Store` implementations or callers that keep `Box<dyn Store>`. Keep the crate dependency-free and compatible with its declared Rust version. Implement the smallest coherent API, document the compatibility choice, and run the project checks.
