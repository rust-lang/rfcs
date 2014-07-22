- Start Date: 2014-07-22
- RFC PR #: 177
- Rust Issue #: 14862

# Summary

If the type of a `static mut` binding, `T`, ascribes to `Share`, then shared
references to the static can be safely acquired.

# Motivation

Today's compiler requires an `unsafe` block for any form of access to a `static
mut` variable. The rationale for this decision is that the current memory model
for Rust dictates that the following two operations are undefined behavior:

1. An unsynchronized write occurring concurrently with any other read/write
    operation.
2. An unsynchronized read occurring concurrently with any other write operation.

If it were safe to both read and write to a `static mut` variable, it would be
possible to trigger undefined behavior in safe code.

This restriction, however, is somewhat limiting. There are a number of
primitives which are appropriate to use as a `static mut` and are entirely safe
in principle. For example:

* `AtomicT`
* `StaticMutex`
* `Once`

Requiring unsafe access to these types unnecessarily introduces `unsafe` blocks
in otherwise safe programs, and have the risk of leading to further unsafe
behaviors if it's unclear what exactly in the block is unsafe.

This RFC attempts to address this concern by allowing safe access to a number
`static mut` variables.

# Detailed design

```rust
static mut FOO: T = ...;
```

If the type `T` ascribes to `Share`, then the compiler will allow code to safely
take a shared borrow of `FOO`. The compiler will still require an `unsafe` block
to take a mutable borrow of `FOO`.

Rust's primary concern is safety, so this proposal must still ensure that any
combination of safe operations will never result in undefined behavior. This
property relies on the definition of
[`Share`](http://doc.rust-lang.org/std/kinds/trait.Share.html):

> a type `T` is `Share` if `&T` is thread-safe. In other words, there is no
> possibility of data races when passing `&T` references between tasks.

The original reason for restricting access to a `static mut` was to prevent data
races between tasks, but from the definition of `Share` it can be seen that
safely allowing shared borrows of `static mut` variables can never lead to data
races.

Data races are still possible, however, if a task writes to `FOO`, hence the
compiler continues to require an `unsafe` block for any write operation.

It should be noted that a large amount of types ascribe to `Share`, which would
imply that the this code snipped is valid:

```rust
fn main() {
    static mut FOO: uint = 1;
    let a: &'static uint = &FOO;
    println!("{}", a); // prints 1
}
```

Here it is seen that the goal of this RFC is not to require *synchronized
reads*, but rather to prevent undefined behavior outlined in the "Motivation"
section.

# Drawbacks

Some types which ascribe to `Share` make it very difficult to invoke undefined
behavior, such as atomics. These types require synchronized reads/writes no
matter the operation.

Other types, such as `uint`, perform no synchronization at all. These types are
safe to read in an unsynchronized fashion, but are unsafe to pair with
concurrent writes. It can be surprising that a read operation requires no
`unsafe` block whereas the write operation does.

# Alternatives

The primary alternative to this proposal is to add a third kind of static,
`static const`.

Currently rust has two kinds of statics, `static` and
`static mut`. The reason for this distinction is to guarantee what statics
are placed in rodata and data sections of executables. In other words,
`static` variables are read-only, and this is enforced by the OS's paging
mechanism, while `static mut` variables are read-write.

With these two statics, anything mutable (including interior mutability like
atomics), *must* be in a `static mut` to prevent a segmentation fault.

A third kind of static, `static const` (or similarly named) could be added
which would require that the type is `Share` and would *only* allow shared
borrows. It would be impossible to get a mutable borrow directly, even via an
unsafe block. The compiler would then continue to require an `unsafe` block to
access a `static mut`, regardless of whether it is a read or a write.

This is seen as introducing unnecessary complexity to the language for not
enough benefit.

# Unresolved questions

None yet.
