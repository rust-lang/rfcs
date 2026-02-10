- Feature Name: `maybe_dropped`
- Start Date: 2026-10-2
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000) (todo)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

A wrapper type for values that may have already been dropped.

This is similar to `mem::MaybeUninit` in that it can represent invalid data when the inner value is not needed, but implies a different lifecycle for the contained value.

Note this type does not provide safe access to `T` and does not run destructors when dropped

## Motivation
[motivation]: #motivation

Some types/systems do not always require the value be active for their entire duration. In these cases, an `Option<T>` is often used.

However, an `Option<T>` here is correct, but the following issues arise:

1. It is misleading here.

While an `Option<T>` works, its not truly an `Option`. For example, the type should not be instantiated as `None`, and the value should only be
`None` after it's usage is completed.

Take this example, from the standard library (simplified)

```rust
pub struct Fuse<I> {
  iter: I
}
```

The actual iterator is not needed after it yields  `None` once, so in this case (not in the standard library, 
however, due to a possible breaking change), We can replace the `I` with a `MaybeDropped`, which can optimize code by dropping the value early
(eg. by freeing allocator space, releasing a lock, etc.)

But you may realize this is exactly the same as using an `Option` (Which `Fuse` does). However, the following point contradicts this.

2. it is more optimized (for size)

`MaybeDropped<T>` has the same memory layout as `ManuallyDrop<T>`, which has the same memory layout as `T`. This can allow optimizations if
there is no need to store the drop state of `T`.

Additionally, types/systems which would need `MaybeDropped` likely already have a means of tracking whether `T` is still needed or not 
elsewhere. Storing it again (for example, using `Option`) is wasteful of memory.

### Why this should be seperate from `MaybeUninit`

`MaybeUninit` implies a value is initialized once and never goes back, having the following lifecycle.

- MaybeUninit created (possibly uninit)
- MaybeUninit is initialized/confirmed to be already initialized.
- it stays initialized.

`MaybeDropped` on the other hand implies the following lifecycle, where data is created in initialized state and later dropped.

- MaybeDropped created initialized or in a `already dropped` state
- The MaybeDropped is used. (if not dropped)
- it is dropped. (if not dropped)
- it stays dropped (not written to)

### Advantages over `T`

For some cases, early drop is *required*; for example:

- Locks
- Allocations (in performance critical code)
- Mechanisms requiring a value is dropped.

`T` does not permit a dropped state, for this reason `MaybeDropped` is used.

### Usage in FFI

`MaybeDropped<T>` is also FFI safe if `T` is FFI safe, this allows for an FFI safe transfer of data that has possibly already been dropped.
This can also allow for an FFI safe `Option`, with additional argmuents

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Usage
Users would use `MaybeDropped` just like a `MaybeUninit`, in reverse.

Take this example:

```rust
struct OnceImage {
    loader: MaybeDropped<ImageLoader>,
    stored: Option<Image> // drop state of `loader` is stored here.
}
```

suppose `ImageLoader` has alot of open resources:

- locks
- OS requests
- File Descriptors
- Services

the `loader` is only used once, while `OnceImage` can be long-lived. It makes no sense here to keep the loader for the lifetime of `OnceImage`.
And `stored` tracks the drop-state of `loader`, so it would be redundant to store it twice.

#### Where to use `MaybeDropped` rather than `MaybeUninit`

The seperation is clear.

`MaybeDropped`: Values which are initialized, and the dropped later.

`MaybeUninit`: Values which may not be initialized yet.

#### Usage in FFI

`MaybeDropped` can be used with `bool` for FFI safe `Option`s

```c
// c code

// extern definition...

void do_some_work() {
    // ...
    if (!dropped) {
        rust_work(possibly_dropped, false)
    } else {
        rust_work(possibly_dropped, true)
    }
}
```
And then, in rust
```rust
// imports...

#[unsafe(no_mangle)]
extern "C" fn rust_work(dropped: MaybeDropped<c_int>, is_dropped: bool) {
    let as_option = if is_dropped {
        None
    } else {
        Some(dropped.assume_alive())
    }
}
```

### Undefined Behaviour in `MaybeDropped`

The following is *__not__* undefined behaviour:

- Creating a `MaybeDropped` that is already dropped. (`MaybeDropped::dropped`)
- Dropping a `MaybeDropped` (so long it is the first time)
- Obtaining raw pointers to the inner memory.

the following *__is__* undefined behaviour:

- Dropping a `MaybeDropped` twice.
- Creating an uninitialized `MaybeDropped`
- any access to the inner memory if it is already dropped.
- Obtaining references to the inner memory (even if not used)
- Writing a value to the `MaybeDropped` after it has already been dropped.

#### Examples
```rust
let dropped = MaybeDropped::<i32>::dropped();
// this is not ok
// let uninit = MaybeUninit::<MaybeDropped<i32>>::uninit().assume_init();

// this is not ok
// let reference = dropped.assume_alive();

// this is ok
let ptr = dropped.as_ptr();

// this fails to compile (not mutable), but is otherwise ok UB-wise
// let mut_ptr = dropped.as_mut_ptr();

unsafe {
    // this is ok
    dropped.assume_alive_drop();
    // this is not
    dropped.assume_alive_drop();
};

// this is not ok
// *ptr.cast_mut() = 42;
// this is not ok
// *ptr
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature's implementation will be similar to `MaybeUninit`

```
union MaybeDropped<T> {
    value: ManuallyDrop<T>,
    dropped: ()
}
```

it does *not* store any drop-state information.

the public API will expose methods to either create it with an initialized value, or create it *logically dropped*.

### Dropping

`MaybeDropped` will not drop on its own.

it must be dropped with `.assume_alive_drop()`

After drop:

- the `MaybeDropped`'s inner value is logically dropped.
- all access is undefined behaviour (both reads and writes)

### Safety

All accesses to the inner value (outside of raw pointers) is `unsafe`, including references.

The inner value must be dropped *at most once*, not dropping is permitted but is considered a leak.

### Interaction with Traits
`MaybeDropped` will not implement any traits which access the inner values (or, more generally, have any safe methods that access the wrapped value)

Trait implementations such as Send and Sync follow the same rules as `MaybeUninit<T>` and are conditional on the corresponding traits of `T`.

### Relation with `MaybeUninit`

MaybeDropped<T> is closely related to mem::MaybeUninit<T>, but represents a distinct lifecycle. While MaybeUninit<T> models memory that may not yet have been initialized, MaybeDropped<T> models memory that was once initialized but may have been destroyed.

This distinction allows low-level code to express post-drop states explicitly without additional storage or ad-hoc flags.

## Drawbacks
[drawbacks]: #drawbacks

- It can already be done with `Option`
- It can already be done with `MaybeUninit`
- It is unnecessarily unsafe (in most cases)
- error prone
  - if the drop-state tracking is not clear, users may accedeintly drop the wrapped value twice.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- `Option<T>`
  - Safer,
  - Less error prone and
  - tracks drop state

 Option wasn't chosen because:

 - it is not FFI safe
 - it is not optimized for size
    
- `MaybeUninit`
  - Already in code
  - supported by external crates.

`MaybeUninit` wasn't chosen because:

- It represents a completely different lifecycle
- its methods dont align with the requirements of `MaybeDropped`

### Impact of not doing this

there isnt much of an impact on regular code, the performance increase is negligble in regular code.

## Prior art
[prior-art]: #prior-art

`MaybeUninit` is already a way, inside of Rust, to represent data that may be invalid.

### mem::MaybeUninit<T>

`mem::MaybeUninit<T>` is the closest existing abstraction to `MaybeDropped<T>`. It represents memory that may not yet be initialized as a valid T and provides no safe access to the underlying value.

While `MaybeUninit<T>` and `MaybeDropped<T>` share similar safety properties, they model different lifecycles. `MaybeUninit<T>` is intended for values that may be initialized at a later point, whereas `MaybeDropped<T>` models values that were once initialized but whose destructor may already have been executed. Conflating these lifecycles would obscure intent and make code harder to reason about.

### mem::ManuallyDrop<T>

`mem::ManuallyDrop<T>` prevents Rust from automatically running the destructor of T while preserving all of T’s invariants. Safe access to the underlying value remains available at all times.

This makes `ManuallyDrop<T>` unsuitable for representing post-drop states, as it assumes the value remains valid even after the destructor is manually invoked. In contrast, `MaybeDropped<T>` explicitly removes any guarantee that the underlying value is valid.

### Option<T>

`Option<T>` is commonly used to model early destruction by replacing a value with `None` once it is no longer needed. This approach is safe and idiomatic in many cases.

However, `Option<T>` introduces additional storage to track the presence of the value and semantically models optional ownership rather than post-drop invalidation. In cases where the drop state is already tracked elsewhere or where layout constraints matter, `Option<T>` is less suitable than `MaybeDropped<T>`.

### Raw pointers and `drop_in_place`

Low-level Rust code can model post-drop states using raw pointers combined with ptr::drop_in_place. This approach is flexible but error-prone, as the drop state is tracked implicitly and must be enforced by convention.

MaybeDropped<T> encapsulates this pattern in a dedicated abstraction, making the intent explicit and reducing the likelihood of accidental misuse.

Additionally, also (usually) force borrowing rather than ownership (outside of `Box`)

### Patterns in existing Rust code

Several low-level Rust abstractions internally rely on patterns similar to `MaybeDropped<T>`, including intrusive data structures, iterator adaptors, and runtime systems that perform early destruction for performance or correctness reasons. These implementations typically use `Option<T>`, `ManuallyDrop<T>`, or raw pointers to represent post-drop states.

`MaybeDropped<T>` provides a more direct and expressive way to model these patterns without additional storage or loss of clarity.

### Other languages and systems

In systems programming contexts outside of Rust, it is common to distinguish between allocation, initialization, and destruction explicitly. Languages and runtimes such as C and C++ permit objects to be destroyed while their storage remains allocated, with correctness enforced by convention.

`MaybeDropped<T>` enables similar low-level control within Rust while preserving its safety model by confining such patterns to unsafe code.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

> - What parts of the design do you expect to resolve through the RFC process before this gets merged?
  
  - What is considered `undefined behaviour` in `MaybeDropped`?

> - What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
  
  - How will we expose the public API?
  - Trait implementations
    - For example, `Unpin`, `Send`, `Sync`?

## Future possibilities
[future-possibilities]: #future-possibilities

### More general Lifetime Types

Currently, we support `uninit` -> `init` (and with this RFC, `init` -> `dropped`), but what if we want to add an abstraction layer over lifetimes as a whole?
Well, thats out of scope for this RFC, but it is a thought.
