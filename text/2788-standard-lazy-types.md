- Feature Name: `once_cell`
- Start Date: 2019-10-17
- RFC PR: [rust-lang/rfcs#2788](https://github.com/rust-lang/rfcs/pull/2788)
- Rust Issue: [rust-lang/rust#74465](https://github.com/rust-lang/rust/issues/74465), [rust-lang/rust#109736](https://github.com/rust-lang/rust/issues/109736), [rust-lang/rust#109737](https://github.com/rust-lang/rust/issues/109737)

# Summary
[summary]: #summary

Add support for lazy initialized values to standard library, effectively superseding the popular [`lazy_static`] crate.

```rust
use std::sync::Lazy;

// `BACKTRACE` implements `Deref<Target = Option<String>>` and is initialized
// on the first access
static BACKTRACE: Lazy<Option<String>> = Lazy::new(|| {
    std::env::var("RUST_BACKTRACE").ok()
});
```

# Motivation
[motivation]: #motivation

Working with lazy initialized values is ubiquitous, [`lazy_static`] and [`lazycell`] crates are used throughout the ecosystem.
Although some of the popularity of `lazy_static` can be attributed to current limitations of constant evaluation in Rust, there are many cases when even perfect `const fn` can't replace lazy values.

At the same time, working with lazy values in Rust is not easy:

* Implementing them requires moderately tricky unsafe code. Multiple soundness holes were found in the implementations from crates.io.
* C++ and Java provide language-level delayed initialization for static values, while Rust requires explicit code to handle runtime-initialization.
* Rust borrowing rules require a special pattern when implementing lazy fields.

`lazy_static` is implemented using macros, to work-around former language limitations. Since then, various language improvements have made it possible to  create runtime initialized (lazy) objects in a `static` scope, accomplishing the same goals without macros.

We can have a single canonical API for a commonly used tricky unsafe concept, so we probably should have it!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Lazy values are a form of interior mutability.
The key observation is that restricting a cell to single assignment allows to safely return a shared reference to the contents of the cell.
Such cell is called `OnceCell`, by analogy with `std::sync::Once` type. The core API is as follows:

```rust
pub struct OnceCell<T> { ... }

impl<T> OnceCell<T> {
    /// Creates a new empty cell.
    pub const fn new() -> OnceCell<T>;

    /// Gets the reference to the underlying value.
    ///
    /// Returns `None` if the cell is empty.
    pub fn get(&self) -> Option<&T>;

    /// Sets the contents of this cell to `value`.
    ///
    /// Returns `Ok(())` if the cell was empty and `Err(value)` if it was
    /// full.
    pub fn set(&self, value: T) -> Result<(), T>;

    /// Gets the contents of the cell, initializing it with `f`
    /// if the cell was empty.
    ///
    /// # Panics
    ///
    /// If `f` panics, the panic is propagated to the caller, and the cell
    /// remains uninitialized.
    ///
    /// It is an error to reentrantly initialize the cell from `f`. Doing
    /// so results in a panic or a deadlock.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    ;

    /// Gets the contents of the cell, initializing it with `f` if
    /// the cell was empty. If the cell was empty and `f` failed, an
    /// error is returned.
    ///
    /// # Panics
    ///
    /// If `f` panics, the panic is propagated to the caller, and the cell
    /// remains uninitialized.
    ///
    /// It is an error to reentrantly initialize the cell from `f`. Doing
    /// so results in a panic or a deadlock.
    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    ;
}
```

Notable features of the API:

* `OnceCell` is created empty, by a const fn.
* Initialization succeeds at most once.
* `get_or_init` and `get_or_try_init` methods can be used to conveniently initialize a cell.
* `get_` family of methods return `&T`.

Similarly to other interior mutability primitives, `OnceCell` comes in two flavors:

* Non thread-safe `std::cell::OnceCell`.
* Thread-safe `std::sync::OnceLock`.

Here's how `OnceCell` can be used to implement lazy-initialized global data:

```rust
use std::{sync::{Mutex, OnceCell}, collections::HashMap};

fn global_data() -> &'static Mutex<HashMap<i32, String>> {
    static INSTANCE: OnceCell<Mutex<HashMap<i32, String>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(13, "Spica".to_string());
        m.insert(74, "Hoyten".to_string());
        Mutex::new(m)
    })
}
```

Here's how `OnceCell` can be used to implement a lazy field:

```rust
use std::{fs, io, path::PathBuf, cell::OnceCell};

struct Ctx {
    config_path: PathBuf,
    config: OnceCell<String>,
}

impl Ctx {
    pub fn get_config(&self) -> Result<&str, io::Error> {
        let cfg = self.config.get_or_try_init(|| {
            fs::read_to_string(&self.config_path)
        })?;
        Ok(cfg.as_str())
    }
}
```

We also provide the more convenient but less powerful `Lazy<T, F>` and `LazyLock<T, F>` wrappers around `OnceCell<T>` and `OnceLock<T>`, which allows specifying the initializing closure at creation time:

```rust
pub struct LazyCell<T, F = fn() -> T> { ... }

impl<T, F: FnOnce() -> T> LazyCell<T, F> {
    /// Creates a new lazy value with the given initializing function.
    pub const fn new(init: F) -> LazyCell<T, F>;

    /// Forces the evaluation of this lazy value and returns a reference to
    /// the result.
    ///
    /// This is equivalent to the `Deref` impl, but is explicit.
    pub fn force(this: &LazyCell<T, F>) -> &T;
}

impl<T, F: FnOnce() -> T> Deref for LazyCell<T, F> {
    type Target = T;

    fn deref(&self) -> &T;
}
```

`LazyLock` directly replaces `lazy_static!`:

```rust
use std::{sync::{Mutex, LazyLock}, collections::HashMap};

static GLOBAL_DATA: LazyLock<Mutex<HashMap<i32, String>>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(13, "Spica".to_string());
    m.insert(74, "Hoyten".to_string());
    Mutex::new(m)
});
```

Moreover, once `#[thread_local]` attribute is stable, `Lazy` might supplant `std::thread_local!` as well:

```rust
use std::cell::{RefCell, Lazy};

#[thread_local]
pub static FOO: Lazy<RefCell<u32>> = Lazy::new(|| RefCell::new(1));
```


Unlike `lazy_static!`, `Lazy` can be used for locals:

```rust
use std::cell::LazyCell;

fn main() {
    let ctx = vec![1, 2, 3];
    let thunk = LazyCell::new(|| {
        ctx.iter().sum::<i32>()
    });
    assert_eq!(*thunk, 6);
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The proposed API is directly copied from [`once_cell`] crate.

Altogether, this RFC proposes to add four types:

* `std::cell::OnceCell`, `std::cell::LazyCell`
* `std::sync::OnceLock`, `std::sync::LazyLock`

`OnceCell` and `OnceLock` are important primitives.
`LazyCell ` and `LazyLock` can be stabilized separately from `OnceCell`, or optionally omitted from the standard library altogether.
However, as they provide significantly nicer ergonomics for the common use case of static lazy values, it is worth developing in tandem.

Non thread-safe flavor is implemented by storing an `UnsafeCell<Option<T>>`:

```rust
pub struct OnceCell<T> {
    // Invariant: written to at most once.
    inner: UnsafeCell<Option<T>>,
}
```

The implementation is mostly straightforward.
The only tricky bit is that reentrant initialization should be explicitly forbidden.
That is, the following program panics:

```rust
let x: OnceCell<Box<i32>> = OnceCell::new();
let dangling_ref: Cell<Option<&i32>> = Cell::new(None);
x.get_or_init(|| {
    let r = x.get_or_init(|| Box::new(92));
    dangling_ref.set(Some(r));
    Box::new(62)
});
println!("would be use after free: {:?}", dangling_ref.get().unwrap());
```

Non thread-safe flavor can be added to `core` as well.

The thread-safe variant is implemented similarly to `std::sync::Once`.
Crucially, it has support for blocking: if many threads call `get_or_init` concurrently, only one will be able to execute the closure, while all other threads will block.
For this reason, most of `std::sync::OnceLock` API can not be provided in `core`.
In the `sync` case, reliably panicking on re-entrant initialization is not trivial.
For this reason, the implementation would simply deadlock, with a note that a deadlock might be elevated to a panic in the future.

# Drawbacks
[drawbacks]: #drawbacks

* This is a moderately large addition to stdlib, there's a chance we do something wrong.
  This can be mitigated by piece-wise stabilization (in particular, `LazyCell` convenience types are optional) and the fact that API is tested in the crates.io ecosystem via `once_cell` crate.

* The design of `LazyCell` type uses default type-parameter as a workaround for the absence of type inference of statics.

* We use the same name for unsync and sync types, which might be confusing.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not `LazyCell` as a primitive?

On the first look, it may seem like we don't need `OnceCell`, and should only provide `LazyCell`.
The critical drawback of `LazyCell` is that it's not always possible to provide the closure at creation time.

This is important for lazy fields:

```rust
struct Ctx {
    config_path: PathBuf,
    config: Lazy<String, ???>,
}

impl Ctx {
    pub fn new(config_path: PathBuf) -> Ctx {
        Ctx {
            config_path,
            config: Lazy::new(|| {
                // We would like to write something like
                // `fs::read_to_string(&self.config_path)`
                // here, but we can't have access to `self`
                ???
            })
        }
    }
}
```

Or for singletons, initialized with parameters:

```rust
use std::{env, io, sync::OnceCell};

#[derive(Debug)]
pub struct Logger { ... }

static INSTANCE: OnceCell<Logger> = OnceCell::new();
impl Logger {
    pub fn global() -> &'static Logger {
        INSTANCE.get().expect("logger is not initialized")
    }
    fn from_cli(args: env::Args) -> Result<Logger, std::io::Error> { ... }
}

fn main() {
    let logger = Logger::from_cli(env::args()).unwrap();

    // Note how we use locally-created value for initialization.
    INSTANCE.set(logger).unwrap();

    // use `Logger::global()` from now on
}
```

## Why `OnceCell` as a primitive?

It is possible to imagine a type, slightly more general than `OnceCell`:

```rust
struct OnceFlipCell<U, V> { ... }

impl<U, V> OnceFlipCell<U, V> {
    const fn new(initial_value: U) -> OnceFlipCell<U, V>;

    fn get_or_init<F: FnOnce(U) -> V>(&self, f: F) -> &V;
}

type OnceCell<T> = OnceFlipCell<(), T>;
```

That is, we can store some initial state in the cell and consume it during initialization.
In practice, such flexibility seems to be rarely required.
Even if we add a type, similar to `OnceFlipCell`, having a dedicated `OnceCell` (which *could* be implemented on top of `OnceFlipCell`) type simplifies a common use-case.

## Variations of `set`

The RFC proposes "obvious" signature for the `set` method:

```rust
fn set(&self, value: T) -> Result<(), T>;
```

Note, however, that `set` establishes an invariant that the cell is initialized, so a more precise signature would be

```rust
fn set(&self, value: T) -> (&T, Option<T>);
```

To be able to return a reference, `set` might need to block a thread.
For example, if two threads call `set` concurrently, one of them needs to block while the other moves the value into the cell.
It is possible to provide a non-blocking alternative to `set`:

```rust
fn try_set(&self, value: T) -> Result<&T, (Option<&T>, T)>
```

That is, if value is set successfully, a reference is returned.
Otherwise, the cell is either fully initialized, and a reference is returned as well, or the cell is being initialized, and no valid reference exist yet.

## Support for `no_std`

The RFC proposes to add `cell::OnceCell` and `cell::LazyCell` to `core`, while keeping `sync::OnceLock` and `sync::LazyLock` `std`-only.
However, there's a subset of `OnceLock` that can be provided in `core`:

```rust
impl<T> OnceCell<T> {
    const fn new() -> OnceCell<T>;
    fn get(&self) -> Option<&T>;
    fn try_set(&self, value: T) -> Result<&T, (Option<&T>, T)>
}
```

It is possible because, while `OnceCell` needs blocking for full API, its internal state can be implemented as a single `AtomicUsize`, so the `core` part does not need to know about blocking.
It is unclear if this API would be significantly useful.
In particular, the guarantees of non-blocking `set` are pretty weak, and are not enough to implement the `Lazy` wrapper.

While it is possible to implement blocking in `#[no_std]` via a spin lock, we explicitly choose not to do so.
Spin locks are a sharp tool, which should only be used in specific circumstances (namely, when you have full control over thread scheduling).
`#[no_std]` code might end up in user space applications with preemptive scheduling, where unbounded spin locks are inappropriate.

A spin-lock based implementation of `OnceCell` is provided on crates.io in [`conquer-once`] crate.

## Poisoning

As a cell can be empty or fully initialized, the proposed API does not use poisoning.
If an initialization function panics, the cell remains uninitialized.
An alternative would be to add poisoning, which will make all subsequent `get` calls to panic.

Similarly, because `OnceCell` provides strong exception safety guarantee, it implements `UnwindSafe`:

```rust
impl<T: UnwindSafe>                    UnwindSafe for OnceCell<T> {}
impl<T: UnwindSafe + RefUnwindSafe> RefUnwindSafe for OnceCell<T> {}
```

## Default type parameter on `Lazy`

`Lazy` is defined with default type parameter.

```rust
pub struct Lazy<T, F = fn() -> T> { ... }
```

This is important to make using `Lazy` in static contexts convenient.
Without this default, the user would have to type `T` type twice:

```rust
static GLOBAL_DATA: Lazy<Mutex<HashMap<i32, String>>, fn() -> Mutex<HashMap<i32, String>>
    = Lazy::new(|| ... );
```

If we allow type inference in statics, this could be shortened to

```rust
static GLOBAL_DATA: Lazy<Mutex<HashMap<i32, String>>, _>
    = Lazy::new(|| ... );
```

There are two drawbacks of using fn pointer type:

* fn pointers are not ZSTs, so we waste one pointer per static lazy value.
  Lazy locals will generally rely on type-inference and will use more specific closure type.
* Specifying type for local lazy value might be tricky: `let x: Lazy<i32> = Lazy::new(|| closed_over_var)` fails with type error, the correct syntax is `let x: Lazy<i32, _> = Lazy::new(|| closed_over_var)`.

## Only thread-safe flavor

It is possible to add only `sync` version of the types, as they are the most useful.
However, this would be against zero cost abstractions spirit.
Additionally, non thread-safe version is required to replace `thread_local!` macro without imposing synchronization.

## Synchronization Guarantees

In theory, it is possible to specify two different synchronization guarantees for `get` operation, release/acquire or release/consume.
They differ in how they treat side effects.
If thread **A** executes `get_or_init(f)`, and thread **B** executes `get` and observes the value, release/acquire guarantees that **B** also observes side-effects of `f`.

Here's a program which allows to observe the difference:

```rust
static FLAG: AtomicBool = AtomicBool::new(false);
static CELL: OnceCell<()> = OnceCell::new();

// thread1
CELL.get_or_init(|| FLAG.store(true, Relaxed));

// thread2
if CELL.get().is_some() {
  assert!(FLAG.load(Relaxed))
}
```

Under release/acquire, the assert never fires.
Under release/consume, it might fire.

Release/consume can potentially be implemented more efficiently on weak memory model architectures.
However, the situation with `consume` ordering is cloudy right now:

* [nobody knows what it actually means](http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2016/p0371r0.html),
* [but people rely on it in practice for performance](https://docs.rs/crossbeam-utils/0.7.0/crossbeam_utils/atomic/trait.AtomicConsume.html#tymethod.load_consume).

Given the cost of `consume` ordering for minimal benefit, this crate proposes to specify and implement `acquire/release` ordering. If at some point Rust adds a `consume/release` option to `std::sync::atomic::Ordering`, the option of adding API methods that accept an `Ordering` can be considered.

# Prior art
[prior-art]: #prior-art

The primary bit of prior art here is the [`once_cell`] library, which itself draws on multiple sources:

* [double-checked-cell](https://crates.io/crates/double-checked-cell)
* [lazy-init](https://crates.io/crates/lazy-init)
* [lazycell](https://crates.io/crates/lazycell)
* [mitochondria](https://crates.io/crates/mitochondria)
* [lazy_static](https://crates.io/crates/lazy_static)

Many languages provide library-defined lazy values, for example [Kotlin](https://kotlinlang.org/api/latest/jvm/stdlib/kotlin/lazy.html#kotlin$lazy(kotlin.Function0((kotlin.lazy.T)))).
Typically, a lazy value is just a wrapper around closure.
This design doesn't always work in Rust, as closing over `self` runs afoul of the borrow checker, we need a more primitive `OnceCell` type.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What is the best naming/place for these types?
- What is the best naming scheme for methods? Is it `get_or_try_init` or `try_insert_with`?
- Is the `F = fn() -> T` hack worth it?
- Which synchronization guarantee should we pick?

# Future possibilities
[future-possibilities]: #future-possibilities

* Once `#[thread_local]` attribute is stable, `cell::Lazy` can serve as a replacement for `std::thread_local!` macro.
* Supporting type inference in constants might allow us to drop the default type parameter on `Lazy`.

[`lazy_static`]: https://crates.io/crates/lazy_static
[`lazycell`]: https://crates.io/crates/lazycell
[`once_cell`]: https://crates.io/crates/once_cell
[`conquer-once`]: https://github.com/oliver-giersch/conquer-once
