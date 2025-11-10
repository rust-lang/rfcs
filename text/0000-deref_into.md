- Feature Name: `deref_into`
- Start Date: 2025-11-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce supertraits `DerefInto` and `DerefMutInto` for `Deref` and `DerefMut` that return their targets by value with independent mutable and immutable targets.

This enables using deref coercion with RAII guards (`RefCell`, `Mutex`, `RwLock`...).

# Motivation
[motivation]: #motivation

Rust's `Deref` and `DerefMut` traits are special because of the "Deref coercion". [Deref coercion is incredibly important for ergonomics. It's so that we can effectively interact with all of the different types of smart pointers as though they were regular ol' references e.g. allowing Box<T> where &T is expected.](https://www.reddit.com/r/rust/comments/1654y5l/comment/jyc9l4x).

However, the standard `Deref`/`DerefMut` traits are limited in that their output types are fixed references (`&Self::Target` / `&mut Self::Target`).

This makes them incompatible with interior mutability or synchronization types such as `RefCell`, `Mutex`, or `RwLock`, which return RAII guards by value (`Ref` / `RefMut` for `RefCell`, `MutexGuard` for `Mutex`, and `RwLockReadGuard` / `RwLockWriteGuard` for `RwLock`).


```rust
pub struct Foo
{
    pub bar: i32,
}

let cell: RefCell<Foo> = RefCell::new(Foo{ bar: 42 });

let _bar = cell.borrow().bar; // Current way to write it

let _bar = &cell.bar; // Error: Impossible to express right now,
// but more ergonomic to write
```

Currently, accessing fields through interior mutability types is verbose, requiring explicit calls to `.borrow()` / `.borrow_mut()` / `.lock()` / `.read()` / `.write()` instead of allowing a natural field/method access syntax.

# Guide-level explanation

Introduce the supertraits `DerefInto` and `DerefMutInto` (for `Deref` and `DerefMut`) to improve ergonomics in the core library:

```rust
trait DerefInto
{
    type Target<'out> where Self: 'out;
    fn deref_into(&self) -> Self::Target<'_>;
}

trait DerefMutInto
{
    type Target<'out> where Self: 'out;
    fn deref_mut_into(&mut self) -> Self::Target<'_>;
}
```

## Example for RefCell

```rust
impl<T> DerefInto for RefCell<T>
{
    type Target<'out> = std::cell::Ref<'out, T> where Self: 'out;
    fn deref_into(&self) -> Self::Target<'_>
    {
        self.borrow()
    }
}
impl<T> DerefMutInto for RefCell<T>
{
    type Target<'out> = std::cell::RefMut<'out, T> where Self: 'out;
    fn deref_mut_into(&mut self) -> Self::Target<'_> {
        self.borrow_mut()
    }
}
```

With these implementations, the following code becomes valid:

```rust
let cell: RefCell<Foo> = RefCell::new(Foo{ bar: 42 });
let bar = &cell.bar;
```

## Example for RAII Singleton access

This approach also enables ergonomic RAII singleton access through zero-sized struct proxies, allowing calls like `MySingleton.foo()` or `MySingleton.foo_mut()` instead of requiring explicit singleton access methods such as `singleton().foo()` or `singleton_mut().foo_mut()`.

__Current way__:

```rust
static SINGLETON: std::sync::RwLock<Foo> = RwLock::new(Foo { bar: 42 });

// public API:
pub struct Foo { pub bar: i32 }

pub fn singleton<'a>() -> RwLockReadGuard<'a, Foo>
{
    SINGLETON.read().unwrap()
}

pub fn singleton_mut<'a>() -> RwLockWriteGuard<'a, Foo>
{
    SINGLETON.write().unwrap()
}

// Current usage:
let bar = singleton().bar;
singleton_mut().bar = 100;
```

__New way__:

```rust
static SINGLETON: std::sync::RwLock<Foo> = RwLock::new(Foo { bar: 42 });

// public API:
pub struct Foo { pub bar: i32 }

pub struct Singleton;

impl DerefInto for Singleton
{
    type Target<'out> = RwLockReadGuard<'out, Foo>;
    fn deref_into(&self) -> Self::Target<'_> {
        SINGLETON.read().unwrap()
    }
}
impl DerefMutInto for Singleton
{
    type Target<'out> = RwLockWriteGuard<'out, Foo>;
    fn deref_mut_into(&mut self) -> Self::Target<'_> {
        SINGLETON.write().unwrap()
    }
}

// New usage:
let bar = &Singleton.bar;
Singleton.bar = 100;
```

*Note:* With plain `Deref` / `DerefMut`, it is possible to create a zero-sized singleton proxy that forwards to a global value, but **RAII-based access is not possible**.

This is because `Deref` and `DerefMut` only allow returning **references** (`&T` / `&mut T`), which cannot carry the RAII lifetime of guards like `RefCell`'s Ref/RefMut or `RwLock`'s read/write guards.

- **Deref / DerefMut**: works for singleton proxies **if the value is just a reference**, but cannot safely return RAII guards.
- **DerefInto / DerefMutInto**: is required for RAII access, because they allow returning the **guard by value**, preserving the lifetime and safety of the borrow or lock.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section introduces the supertraits `DerefInto` and `DerefMutInto` for `Deref` and `DerefMut` to improve ergonomics:

```rust
trait DerefInto
{
    type Target<'out> where Self: 'out;
    fn deref_into(&self) -> Self::Target<'_>;
}

trait DerefMutInto
{
    type Target<'out> where Self: 'out;
    fn deref_mut_into(&mut self) -> Self::Target<'_>;
}
```

These traits provide **cheap, ergonomic access to RAII-guarded types** without explicit `.borrow()` or `.lock()` calls.
They are compatible with the standard `Deref`/`DerefMut` trait because they can return either a **RAII guard by value** or a **plain reference**, preserving the expected deref semantics.

## Backward compatibility

These traits are intended to be the only traits that support Deref coercion.
The `Deref` and `DerefMut` trait implementation of the core library will just delegate to `DerefInto` and `DerefMutInto`:

```rust
impl<T> DerefInto for T where T: Deref
{
    type Target<'out> = &'out T::Target where Self: 'out;
    fn deref_into(&self) -> Self::Target<'_> {
        self.deref()
    }
}
impl<T> DerefMutInto for T where T: DerefMut
{
    type Target<'out> = &'out T::Target where Self: 'out;
    fn deref_mut_into(&mut self) -> Self::Target<'_> {
        self.deref_mut()
    }
}
```

This ensures full backward compatibility, while enabling ergonomic, by-value RAII access for interior mutability or synchronization types.

# Drawbacks
[drawbacks]: #drawbacks

This adds complexity to the `Deref` and `DerefMut` APIs, especially since the `DerefInto` and `DerefMutInto` Target types may differ and support the deref coercion feature.

Implementing them on `Mutex` or `RwLock` may be undesirable because:

-  The `.lock()` / `.read()` / `.write()` can fail, so the `DerefInto`/`DerefMutInto` implementation may panic. (The `Index`/`IndexMut` trait work in a similar way, panic on invalid indices via [`index()`](https://doc.rust-lang.org/std/ops/trait.Index.html#tymethod.index)).

-  Most notably, if `DerefInto` and `DerefMutInto` are implemented for `Mutex` or `RwLock`, it's not clear how these trait should behave on poisoned state.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

RAII access by value is only possible using `DerefInto` / `DerefMutInto`, not with standard `Deref` / `DerefMut`.

This design extends the existing `Deref` / `DerefMut` trait, preserving their semantics while generalizing their output type to support by-value dereferencing.

It also makes the language more expressive and ergonomic while also preserving performance and safety, and backward compatibility.

This feature cannot be implemented purely as a library or macro, since Deref coercion is a compiler-level behavior. Extending it to support by-value outputs requires language support.

I hope that a mechanism similar to the proposed `DerefInto` and `DerefMutInto` will be available initially.

The implementation of these traits for existing types such as `RefCell`, `Mutex`, and `RwLock` could be addressed later, rejected or not, or considered out of scope for this initial RFC.

Since this feature primarily benefits user-defined types and library authors seeking more ergonomic access patterns, native support in the core library can reasonably come at a later stage once the mechanism is stable.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- `DerefMut` implies `Deref`, but can `DerefMutInto` be implemented without `DerefInto` ?

- If the `DerefInto` implementation is pure, does that mean the target type is clonable?

- If both traits are implemented and their target types differ, how should name resolution work? Should the resolver first look into the `DerefInto` target for the field or method, and then fall back to the `DerefMutInto` target if not found?

- Remove the reference in the parameter to make it more flexible ?
ex:
```rust
trait DerefIntoByValue
{
    type Target;
    fn deref_into(self) -> Self::Target;
}
```
In that case:

- There is no need for the `DerefMutIntoByValue` trait because `DerefIntoByValue` can be implemented for `&mut T`.

- The `DerefIntoByValue` trait look really similar to `Into`, except it is not generic and use a GAT. Does `DerefIntoByValue` imply `Into<Self::Target>`?

# Future possibilities
[future-possibilities]: #future-possibilities

No additional future possibilities are identified at this time.
