---
name: rust
description: Use this skill when writing or reviewing any Rust code in this project. Enforces ownership patterns, error handling, performance, async patterns, and anti-pattern avoidance compiled from multiple expert sources.
---

# Rust Engineering Standards

Apply these rules whenever writing or reviewing Rust code in this project.

## Ownership & Borrowing

- Prefer references over cloning. Use `&str` and `&[T]` in function signatures, not owned `String`/`Vec`.
- Small `Copy` types (≤24 bytes) pass by value; larger types pass by reference.
- Apply `Cow<'_, T>` when ownership depends on the call site.
- Justify every `.clone()` — if you can't, restructure ownership instead.
- When fighting lifetimes, the root cause is usually a data structure problem; restructure rather than annotate around it.

## Error Handling

- Return `Result<T, E>` for all fallible operations. No panics in production paths.
- Never use `.unwrap()` in non-test code. Use `.expect("reason")` only when the invariant is truly guaranteed and document why.
- Use `?` for error propagation.
- Use `thiserror` for library/component error types; `anyhow` for application-level error context.
- Never ignore `#[must_use]` warnings — handle the `Result`/`Option` explicitly.

## Performance

- Benchmark only with `--release`; debug builds are meaningless for perf.
- Prefer iterator chains (`.map()`, `.filter()`, `.fold()`) over manual index loops.
- Avoid `.collect()` in the middle of a chain unless you need a concrete collection.
- Avoid loops that clone; use `.iter()` for `Copy` types.
- Static dispatch (generics + monomorphization) for hot paths; `dyn Trait` only for heterogeneous collections where code size matters more than speed.

## Anti-Patterns to Avoid

| Pattern | Why it's wrong | Fix |
|---|---|---|
| `.clone()` to escape borrow checker | Hides ownership design flaw | Restructure lifetimes |
| `.unwrap()` in production | Runtime panic | `?` or explicit match |
| `Rc` with single owner | Unnecessary overhead | Plain ownership |
| `unsafe` for convenience | UB risk | Find safe API |
| `Deref` coercion for OOP inheritance | Misleading API surface | Composition + traits |
| Giant `match` arms | Hard to maintain | Extract to helper fns |
| `String` everywhere | Wasteful allocation | `&str` or `Cow<str>` |
| Index-based loops | Panics, off-by-one | Iterators |

## Async Patterns (Tokio)

- Use `JoinSet` for dynamic sets of spawned tasks; `select!` for racing futures.
- Channel choice: `mpsc` for queues, `broadcast` for fan-out events, `oneshot` for single responses, `watch` for latest-value state.
- Never hold a `Mutex`/`RwLock` lock across an `.await` point — deadlocks or starvation.
- Never call blocking code (file I/O, `std::thread::sleep`, CPU-heavy loops) inside an async task without `spawn_blocking`.
- Bound all task spawning — unbounded spawning exhausts memory.
- Use `CancellationToken` for coordinated graceful shutdown.
- Instrument async code with `#[tracing::instrument]`; use `tokio-console` for runtime introspection.

## Traits & Type Design

- Encode valid states in the type system (type-state pattern) to make illegal states unrepresentable.
- Prefer associated types over generic parameters when there is only one sensible implementation per type.
- Use composition and explicit trait impls rather than `Deref` for sharing behaviour.

## Code Quality

- Keep functions under 50 lines. If longer, extract named helpers.
- Keep fields private; expose via methods.
- Include `SAFETY:` comments on every `unsafe` block explaining the invariant.
- Add issue links to TODOs: `// TODO(#42): ...`
- Write doc comments (`///`) for all public items, with a usage example.
- Use `#[expect(clippy::lint)]` + justification instead of bare `#[allow(...)]`.

## Linting (run before every commit)

```
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check
```

Priority lints to never suppress without justification: `redundant_clone`, `large_enum_variant`, `needless_collect`, `unwrap_used`.

## Testing

- Test names should describe expected behaviour: `fn returns_error_when_input_is_empty()`.
- One logical assertion per test where possible.
- Unit tests for pure logic; integration tests for system boundaries.
- Doctests for public API examples — they double as documentation.
