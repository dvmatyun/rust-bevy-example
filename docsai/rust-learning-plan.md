# Rust learning plan (general — beyond this project)

Topics roughly ordered by "what unblocks reading the next thing".
Skim the headings; jump to the topic you've just felt confused about.

> **Most useful book by far**: *The Rust Programming Language* —
> [doc.rust-lang.org/book](https://doc.rust-lang.org/book/). Often
> called "The Book". The chapter numbers below refer to it.

## 1. Ownership and borrowing — the unique Rust thing

The thing that makes Rust feel different from every other language.

- **Ownership**: every value has exactly one owner. When the owner goes
  out of scope, the value is freed. No GC, no manual `free`.
- **Move semantics**: `let b = a;` *moves* `a` into `b` (for non-`Copy`
  types). `a` is no longer usable.
- **Borrowing**: `&a` is a shared (read-only) borrow. `&mut a` is an
  exclusive (read/write) borrow. You can have many shared borrows OR
  one mutable borrow at a time, never both.
- **Lifetimes (`'a`)**: a name for "how long is this borrow valid".
  Mostly inferred; explicit only when the compiler can't tell.
- **`Copy` vs `Clone`**: types that are cheap to copy bit-for-bit (`i32`,
  `f32`, `bool`, `[T; N]` of `Copy` types) implement `Copy` — assignment
  duplicates instead of moving. `Clone` is the explicit `.clone()`
  function for non-`Copy` types like `String` or `Vec<T>`.

**Read** — Book chapters 4 ("Understanding Ownership") and 10 ("Generic
Types, Traits, and Lifetimes" — the lifetime sub-chapter).
**Practice** — write a function that takes a `&str` and a `&mut String`
and appends. The compiler will yell at you in instructive ways.

## 2. Pattern matching, `Option`, `Result`, the `?` operator

How Rust represents "maybe nothing" and "maybe an error".

- `Option<T>` = `Some(T) | None`. There's no null.
- `Result<T, E>` = `Ok(T) | Err(E)`. There's no exception.
- `match` exhaustively pattern-matches enum variants.
- `if let Some(x) = opt { ... }` is sugar for matching one variant.
- `let Some(x) = opt else { return; }` ("let-else") is sugar for
  "match-or-early-return", which appears all over this codebase.
- `?` propagates errors: `let x = thing()?;` is "try thing(); if Err,
  return that error from this function".

**Read** — Book chapters 6, 9, and 18.
**Practice** — write a function `fn parse_pair(s: &str) -> Option<(i32, i32)>`
that parses `"3,5"`. Use `?` and `Option::and_then`.

## 3. Traits, generics, trait bounds

Polymorphism in Rust. The `derive` you keep seeing comes from here.

- **Trait** = a set of method signatures, like an interface.
- **Generic types** (`fn f<T>(x: T)`) work on any type that satisfies
  the bounds.
- **Trait bounds** (`fn f<T: Display>(x: T)`) restrict generics: this
  function only takes `T`s that implement `Display`.
- **`impl Trait`** in argument or return position = "some specific type
  implementing this trait". Often clearer than naming the type.
- **`dyn Trait`** = "a runtime-dispatched trait object". Slower than
  generics but flexible. You'll see `Box<dyn Plugin>`, `&dyn Iterator`.
- **`#[derive(...)]`** invokes built-in macros that generate trait
  implementations from your struct definition. `#[derive(Debug, Clone,
  Default, Component)]` writes those impls for you.

**Read** — Book chapter 10. Also skim chapter 17 for trait objects.
**Practice** — define a `Greet` trait with a `greet(&self) -> String`
method, implement it for two structs, write a function `fn shout<T:
Greet>(x: T)`.

## 4. Smart pointers: `Box`, `Rc`, `Arc`, `RefCell`, `Mutex`

When you need shared ownership or interior mutability.

- **`Box<T>`**: heap-allocated `T`. Owned. Single owner. Used to
  break recursive types or to use trait objects.
- **`Rc<T>`** ("reference counted"): shared ownership in single-threaded
  code. `Rc::clone(&rc)` increments the counter cheaply.
- **`Arc<T>`** ("atomic reference counted"): same, but thread-safe. Use
  whenever Bevy / async / threads are involved.
- **`RefCell<T>`**: lets you mutate inside a shared borrow at the cost
  of a runtime borrow check. Single-threaded.
- **`Mutex<T>` / `RwLock<T>`**: thread-safe interior mutability.
- **`Cow<'_, T>`** ("clone on write"): borrows until you want to
  modify, then clones lazily. Used by deserialisers.

**Read** — Book chapter 15.
**Practice** — write a doubly-linked list using `Rc<RefCell<Node>>`. (Or
use `Vec` like a normal person; the exercise is to feel why doubly-
linked needs both.)

## 5. Iterators and closures

Functional programming primitives. Almost every loop you'd write in
another language is an iterator chain in Rust.

- **Iterator** = anything implementing `Iterator { type Item; fn next(&mut self) -> Option<Self::Item>; }`.
- Methods like `.map()`, `.filter()`, `.collect()`, `.fold()`, `.take()`,
  `.zip()` are *adapters* that return new iterators (lazy).
- Lazy means nothing happens until a *terminal* method runs (`.collect()`,
  `.count()`, `.for_each()`, `for x in iter`).
- **Closures** are anonymous functions with captures: `|x| x + 1` or
  `move |x| { state.push(x) }`. They implement `Fn`, `FnMut`, or
  `FnOnce` depending on how they capture.

**Read** — Book chapters 13 and 19 (more on closures).
**Practice** — given a `Vec<i32>`, get the sum of squares of even values
using `.iter().filter(...).map(...).sum()`. No `for` loop.

## 6. Error handling: `thiserror`, `anyhow`

Beyond the basic `Result`.

- **`anyhow::Result<T>`** is the application-level "any error wrapped"
  type. Convenient when you don't care about the specific error variant.
- **`thiserror`** is the library-level helper for declaring your own
  error enums with `#[derive(thiserror::Error)]`. Each variant gets a
  `Display` message and `From` conversions for free.
- The pattern: libraries return concrete `Result<T, MyLibError>` (use
  `thiserror`), applications consume them with `Result<T, anyhow::Error>`
  (use `anyhow`).

**Read** — [`thiserror` docs](https://docs.rs/thiserror), [`anyhow` docs](https://docs.rs/anyhow).
**Practice** — convert a function returning `Result<T, String>` to
return `Result<T, MyError>` with a custom `MyError` enum.

## 7. Async: `Future`, `tokio`, `async/await`

Rust's async model. Different from JS / Python — no event loop is
built in; you pick a runtime.

- An `async fn` returns a `Future<Output = T>`. The future does
  nothing on its own; you `.await` it.
- `tokio` is the most common runtime. It runs many `Future`s on a
  thread pool.
- Send a future to be run with `tokio::spawn(async move { ... })`.
- `select!` waits for the first of several futures to complete.
- `.await` is a *suspension point*: the function pauses, control
  returns to the runtime, and resumes when the awaited future is
  ready.

**Read** — [Async Book](https://rust-lang.github.io/async-book/) chapters 1–4. Skip the rest until you need it.
**Practice** — read a file with `tokio::fs::read_to_string(path).await`.

(Bevy is *not* async — it has its own scheduler. Async Rust is more
relevant for networking and IO.)

## 8. Memory layout: stack vs heap, `Vec` internals, `repr`

What Rust types actually look like in memory.

- **Stack**: small, fast, fixed-size, freed at scope end. Local
  variables, primitive types, `[T; N]`, structs of those.
- **Heap**: larger, slower to allocate, dynamic. `Box<T>`, `Vec<T>`,
  `String`, `HashMap`, `Rc/Arc<T>`.
- **`Vec<T>`** = `(ptr_to_heap_buffer, length, capacity)`. Three
  machine words. The buffer is heap-allocated.
- **`String`** = `Vec<u8>` with an invariant of valid UTF-8.
- **`Box<T>`** = a single pointer to a heap allocation.
- **`#[repr(C)]`** forces a struct's layout to match C's. Useful for
  FFI.
- **Niche optimisation**: `Option<&T>` and `Option<Box<T>>` are the
  same size as `&T` / `Box<T>` because `None` is encoded as a null
  pointer. `Option<bool>` is 1 byte. `Option<NonZeroU32>` is 4 bytes.

**Read** — [The Rustonomicon](https://doc.rust-lang.org/nomicon/) chapters 1–3 (memory and unsafe basics) when you're ready for serious detail.
**Practice** — `std::mem::size_of::<Vec<u8>>()` is 24 on 64-bit. Why? Run it.

## 9. Macros: `macro_rules!` and `#[derive]`

Why so many `#[...]` attributes everywhere.

- **`macro_rules!`** is the simple declarative macro system. They
  pattern-match token streams and expand to other token streams. Used
  for things like `vec![1, 2, 3]` and `println!("{}", x)`.
- **Procedural macros** are Rust code that runs at compile time and
  generates Rust code. The big three:
  1. `#[derive(...)]` — generates trait implementations from a struct.
  2. **Function-like** procedural macros — like `macro_rules!` but
     written in Rust (e.g. `sqlx::query!`).
  3. **Attribute-like** procedural macros — wrap a function or struct
     and rewrite it (e.g. `#[tokio::main]`, `#[bevy_main]`).
- Bevy uses `#[derive(Component)]`, `#[derive(Resource)]`, `#[derive(Bundle)]`,
  `#[derive(SystemSet)]`, `#[bevy_main]` — all procedural macros.

**Read** — Book chapter 19 (the macro section). Skip writing macros
unless / until you really need them.

## 10. Cargo: workspaces, profiles, features

The package manager.

- **`cargo build`**, **`cargo run`**, **`cargo test`**, **`cargo check`**
  — `check` doesn't link the binary; fastest way to find errors.
- **`cargo build --release`** turns on optimisations. Massive speed
  improvement, slow to compile.
- **Workspaces**: a top-level `Cargo.toml` with `[workspace] members =
  ["a", "b"]` lets multiple crates share a `target/` and version lock.
  This project is a workspace.
- **Profiles**: `[profile.dev]` and `[profile.release]` in
  `Cargo.toml` tune optimisations / debug info. We have
  `strip = "debuginfo"` for Android.
- **Features**: opt-in compile-time flags (`features = ["foo"]`).
  `default-features = false` opts out of default features. Bevy uses
  this aggressively.

**Read** — [Cargo book](https://doc.rust-lang.org/cargo/) chapters 1–3 and 6.
**Practice** — add a feature flag to a small library and use it to
gate a function.

## 11. `unsafe` Rust — only if you need it

Almost all application code (including this game) avoids `unsafe`.
Read up only when you start FFI, custom data structures, or
performance-critical zero-cost abstractions.

- `unsafe { ... }` lets you do five forbidden things: dereference raw
  pointers, call unsafe functions, access mutable statics, implement
  unsafe traits, access fields of unions.
- The contract: *you* prove the operation is sound; the compiler
  trusts you. If you're wrong, you get UB.
- Most Rust crates wrap one or two `unsafe` blocks behind a safe API.

**Read** — [The Rustonomicon](https://doc.rust-lang.org/nomicon/) when you're committed.

## Recommended reading order for a beginner

1. Skim *The Book* chapters 1–6 for syntax basics.
2. Do **Rustlings** ([github.com/rust-lang/rustlings](https://github.com/rust-lang/rustlings)) — small exercises that drill core concepts.
3. Read this codebase's [`learning-plan.md`](learning-plan.md) to see Rust applied to an actual project.
4. *The Book* chapters 4, 6, 9, 10, 13, 15 (in that order — they're independent).
5. *Rust by Example* ([doc.rust-lang.org/rust-by-example](https://doc.rust-lang.org/rust-by-example/)) for "show me, don't tell me".
6. *Rust for Rustaceans* (Jon Gjengset) — the next-level book once you can read most of the standard library.
7. *Rust Atomics and Locks* (Mara Bos) — when you need to write your own concurrent data structures.

## Tools you'll want

- **`cargo clippy`** — extra lints; run before every commit.
- **`cargo fmt`** — formats code. Set up your editor to run it on save.
- **`cargo doc --open`** — opens the local rustdocs; the standard
  library docs are excellent.
- **`rustup component add rust-src`** — lets `rust-analyzer` jump into
  std source.

## What to skip until later

- Procedural macros (write them only when nothing else fits).
- `Pin`, `!Unpin`, async internals — only if you're implementing your
  own runtime or async data structure.
- `std::sync::atomic` low-level orderings — only when you're writing
  lock-free code.
- `unsafe` of any kind — almost never needed in app code.
