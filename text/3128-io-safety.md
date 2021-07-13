- Feature Name: `io_safety`
- Start Date: 2021-05-24
- RFC PR: [rust-lang/rfcs#3128](https://github.com/rust-lang/rfcs/pull/3128)
- Rust Issue: [rust-lang/rust#87074](https://github.com/rust-lang/rust/issues/87074)

# Summary
[summary]: #summary

Close a hole in encapsulation boundaries in Rust by providing users of
`AsRawFd` and related traits guarantees about their raw resource handles, by
introducing a concept of *I/O safety* and a new set of types and traits.

# Motivation
[motivation]: #motivation

Rust's standard library almost provides *I/O safety*, a guarantee that if one
part of a program holds a raw handle privately, other parts cannot access it.
[`FromRawFd::from_raw_fd`] is unsafe, which prevents users from doing things
like `File::from_raw_fd(7)`, in safe Rust, and doing I/O on a file descriptor
which might be held privately elsewhere in the program.

However, there's a loophole. Many library APIs use [`AsRawFd`]/[`IntoRawFd`] to
accept values to do I/O operations with:

```rust
pub fn do_some_io<FD: AsRawFd>(input: &FD) -> io::Result<()> {
    some_syscall(input.as_raw_fd())
}
```

`AsRawFd` doesn't restrict `as_raw_fd`'s return value, so `do_some_io` can end
up doing I/O on arbitrary `RawFd` values. One can even write `do_some_io(&7)`,
since [`RawFd`] itself implements `AsRawFd`.

This can cause programs to [access the wrong resources], or even break
encapsulation boundaries by creating aliases to raw handles held privately
elsewhere, causing [spooky action at a distance].

And in specialized circumstances, violating I/O safety could even lead to
violating memory safety. For example, in theory it should be possible to make
a safe wrapper around an `mmap` of a file descriptor created by Linux's
[`memfd_create`] system call and pass `&[u8]`s to safe Rust, since it's an
anonymous open file which other processes wouldn't be able to access. However,
without I/O safety, and without permanently sealing the file, other code in
the program could accidentally call `write` or `ftruncate` on the file
descriptor, breaking the memory-safety invariants of `&[u8]`.

This RFC introduces a path to gradually closing this loophole by introducing:

 - A new concept, I/O safety, to be documented in the standard library
   documentation.
 - A new set of types and traits.
 - New documentation for
   [`from_raw_fd`]/[`from_raw_handle`]/[`from_raw_socket`] explaining why
   they're unsafe in terms of I/O safety, addressing a question that has
   come up a [few] [times].

[few]: https://github.com/rust-lang/rust/issues/72175
[times]: https://users.rust-lang.org/t/why-is-fromrawfd-unsafe/39670
[access the wrong resources]: https://cwe.mitre.org/data/definitions/910.html
[spooky action at a distance]: https://en.wikipedia.org/wiki/Action_at_a_distance_(computer_programming)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The I/O safety concept

Rust's standard library has low-level types, [`RawFd`] on Unix-like platforms,
and [`RawHandle`]/[`RawSocket`] on Windows, which represent raw OS resource
handles. These don't provide any behavior on their own, and just represent
identifiers which can be passed to low-level OS APIs.

These raw handles can be thought of as raw pointers, with similar hazards.
While it's safe to *obtain* a raw pointer, *dereferencing* a raw pointer could
invoke undefined behavior if it isn't a valid pointer or if it outlives the
lifetime of the memory it points to. Similarly, it's safe to *obtain* a raw
handle, via [`AsRawFd::as_raw_fd`] and similar, but using it to do I/O could
lead to corrupted output, lost or leaked input data, or violated encapsulation
boundaries, if it isn't a valid handle or it's used after the `close` of its
resource. And in both cases, the effects can be non-local, affecting otherwise
unrelated parts of a program. Protection from raw pointer hazards is called
memory safety, so protection from raw handle hazards is called *I/O safety*.

Rust's standard library also has high-level types such as [`File`] and
[`TcpStream`] which are wrappers around these raw handles, providing high-level
interfaces to OS APIs.

These high-level types also implement the traits [`FromRawFd`] on Unix-like
platforms, and [`FromRawHandle`]/[`FromRawSocket`] on Windows, which provide
functions which wrap a low-level value to produce a high-level value. These
functions are unsafe, since they're unable to guarantee I/O safety. The type
system doesn't constrain the handles passed in:

```rust
    use std::fs::File;
    use std::os::unix::io::FromRawFd;

    // Create a file.
    let file = File::open("data.txt")?;

    // Construct a `File` from an arbitrary integer value. This type checks,
    // however 7 may not identify a live resource at runtime, or it may
    // accidentally alias encapsulated raw handles elsewhere in the program. An
    // `unsafe` block acknowledges that it's the caller's responsibility to
    // avoid these hazards.
    let forged = unsafe { File::from_raw_fd(7) };

    // Obtain a copy of `file`'s inner raw handle.
    let raw_fd = file.as_raw_fd();

    // Close `file`.
    drop(file);

    // Open some unrelated file.
    let another = File::open("another.txt")?;

    // Further uses of `raw_fd`, which was `file`'s inner raw handle, would be
    // outside the lifetime the OS associated with it. This could lead to it
    // accidentally aliasing other otherwise encapsulated `File` instances,
    // such as `another`. Consequently, an `unsafe` block acknowledges that
    // it's the caller's responsibility to avoid these hazards.
    let dangling = unsafe { File::from_raw_fd(raw_fd) };
```

Callers must ensure that the value passed into `from_raw_fd` is explicitly
returned from the OS, and that `from_raw_fd`'s return value won't outlive the
lifetime the OS associates with the handle.

I/O safety is new as an explicit concept, but it reflects common practices.
Rust's `std` will require no changes to stable interfaces, beyond the
introduction of some new types and traits and new impls for them. Initially,
not all of the Rust ecosystem will support I/O safety though; adoption will
be gradual.

## `OwnedFd` and `BorrowedFd<'fd>`

These two types are conceptual replacements for `RawFd`, and represent owned
and borrowed handle values. `OwnedFd` owns a file descriptor, including closing
it when it's dropped. `BorrowedFd`'s lifetime parameter says for how long
access to this file descriptor has been borrowed. These types enforce all of
their I/O safety invariants automatically.

For Windows, similar types, but in `Handle` and `Socket` forms.

These types play a role for I/O which is analogous to what existing types
in Rust play for memory:

| Type             | Analogous to |
| ---------------- | ------------ |
| `OwnedFd`        | `Box<_>`     |
| `BorrowedFd<'a>` | `&'a _`      |
| `RawFd`          | `*const _`   |

One difference is that I/O types don't make a distinction between mutable
and immutable. OS resources can be shared in a variety of ways outside of
Rust's control, so I/O can be thought of as using [interior mutability].

[interior mutability]: https://doc.rust-lang.org/reference/interior-mutability.html

## `AsFd`, `Into<OwnedFd>`, and `From<OwnedFd>`

These three are conceptual replacements for `AsRawFd::as_raw_fd`,
`IntoRawFd::into_raw_fd`, and `FromRawFd::from_raw_fd`, respectively,
for most use cases. They work in terms of `OwnedFd` and `BorrowedFd`, so
they automatically enforce their I/O safety invariants.

Using these, the `do_some_io` example in the [motivation] can avoid the
original problems. Since `AsFd` is only implemented for types which properly
own or borrow their file descriptors, this version of `do_some_io` doesn't
have to worry about being passed bogus or dangling file descriptors:

```rust
pub fn do_some_io<FD: AsFd>(input: &FD) -> io::Result<()> {
    some_syscall(input.as_fd())
}
```

For Windows, similar traits, but in `Handle` and `Socket` forms.

## Gradual adoption

I/O safety and the new types and traits wouldn't need to be adopted
immediately; adoption could be gradual:

 - First, `std` adds the new types and traits with impls for all the relevant
   `std` types. This is a backwards-compatible change.

 - After that, crates could begin to use the new types and implement the new
   traits for their own types. These changes would be small and semver-compatible,
   without special coordination.

 - Once the standard library and enough popular crates implement the new
   traits, crates could start to switch to using the new traits as bounds when
   accepting generic arguments, at their own pace. These would be
   semver-incompatible changes, though most users of APIs switching to these
   new traits wouldn't need any changes.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## The I/O safety concept

In addition to the Rust language's memory safety, Rust's standard library also
guarantees I/O safety. An I/O operation is *valid* if the raw handles
([`RawFd`], [`RawHandle`], and [`RawSocket`]) it operates on are values
explicitly returned from the OS, and the operation occurs within the lifetime
the OS associates with them. Rust code has *I/O safety* if it's not possible
for that code to cause invalid I/O operations.

While some OS's document their file descriptor allocation algorithms, a handle
value predicted with knowledge of these algorithms isn't considered "explicitly
returned from the OS".

Functions accepting arbitrary raw I/O handle values ([`RawFd`], [`RawHandle`],
or [`RawSocket`]) should be `unsafe` if they can lead to any I/O being
performed on those handles through safe APIs.

## `OwnedFd` and `BorrowedFd<'fd>`

`OwnedFd` and `BorrowedFd` are both `repr(transparent)` with a `RawFd` value
on the inside, and both can use niche optimizations so that `Option<OwnedFd>`
and `Option<BorrowedFd<'_>>` are the same size, and can be used in FFI
declarations for functions like `open`, `read`, `write`, `close`, and so on.
When used this way, they ensure I/O safety all the way out to the FFI boundary.

These types also implement the existing `AsRawFd`, `IntoRawFd`, and `FromRawFd`
traits, so they can interoperate with existing code that works with `RawFd`
types.

## `AsFd`, `Into<OwnedFd>`, and `From<OwnedFd>`

These types provide `as_fd`, `into`, and `from` functions similar to
`AsRawFd::as_raw_fd`, `IntoRawFd::into_raw_fd`, and `FromRawFd::from_raw_fd`,
respectively.

## Prototype implementation

All of the above is prototyped here:

<https://github.com/sunfishcode/io-lifetimes>

The README.md has links to documentation, examples, and a survey of existing
crates providing similar features.

# Drawbacks
[drawbacks]: #drawbacks

Crates with APIs that use file descriptors, such as [`nix`] and [`mio`], would
need to migrate to types implementing `AsFd`, or change such functions to be
unsafe.

Crates using `AsRawFd` or `IntoRawFd` to accept "any file-like type" or "any
socket-like type", such as [`socket2`]'s [`SockRef::from`], would need to
either switch to `AsFd` or `Into<OwnedFd>`, or make these functions unsafe.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Concerning "unsafe is for memory safety"

Rust historically drew a line in the sand, stating that `unsafe` would only
be for memory safety. A famous example is [`std::mem::forget`], which was
once `unsafe`, and was [changed to safe]. The conclusion stating that unsafe
only be for memory safety observed that unsafe should not be for “footguns”
or for being “a general deterrent for "should be avoided" APIs”.

Memory safety is elevated above other programming hazards because it isn't
just about avoiding unintended behavior, but about avoiding situations where
it's impossible to bound the set of things that a piece of code might do.

I/O safety is also in this category, for two reasons.

 - I/O safety errors can lead to memory safety errors in the presence of
   safe wrappers around `mmap` (on platforms with OS-specific APIs allowing
   them to otherwise be safe).

 - I/O safety errors can also mean that a piece of code can read, write, or
   delete data used by other parts of the program, without naming them or
   being given a reference to them. It becomes impossible to bound the set
   of things a crate can do without knowing the implementation details of all
   other crates linked into the program.

Raw handles are much like raw pointers into a separate address space; they can
dangle or be computed in bogus ways. I/O safety is similar to memory safety;
both prevent spooky-action-at-a-distance, and in both, ownership is the main
foundation for robust abstractions, so it's natural to use similar safety
concepts.

[`std::mem::forget` being safe]: https://doc.rust-lang.org/std/mem/fn.forget.html
[changed to safe]: https://rust-lang.github.io/rfcs/1066-safe-mem-forget.html

## I/O Handles as plain data

The main alternative would be to say that raw handles are plain data, with no
concept of I/O safety and no inherent relationship to OS resource lifetimes. On
Unix-like platforms at least, this wouldn't ever lead to memory unsafety or
undefined behavior.

However, most Rust code doesn't interact with raw handles directly. This is a
good thing, independently of this RFC, because resources ultimately do have
lifetimes, so most Rust code will always be better off using higher-level types
which manage these lifetimes automatically and which provide better ergonomics
in many other respects. As such, the plain-data approach would at best make raw
handles marginally more ergonomic for relatively uncommon use cases. This would
be a small benefit, and may even be a downside, if it ends up encouraging people
to write code that works with raw handles when they don't need to.

The plain-data approach also wouldn't need any code changes in any crates. The
I/O safety approach will require changes to Rust code in crates such as
[`socket2`], [`nix`], and [`mio`] which have APIs involving [`AsRawFd`] and
[`RawFd`], though the changes can be made gradually across the ecosystem rather
than all at once.

## The `IoSafe` trait (and `OwnsRaw` before it)

Earlier versions of this RFC proposed an `IoSafe` trait, which was meant as a
minimally intrusive fix. Feedback from the RFC process led to the development
of a new set of types and traits. This has a much larger API surface area,
which will take more work to design and review. And it and will require more
extensive changes in the crates ecosystem over time. However, early indications
are that the new types and traits are easier to understand, and easier and
safer to use, and so are a better foundation for the long term.

Earlier versions of `IoSafe` were called `OwnsRaw`. It was difficult to find a
name for this trait which described exactly what it does, and arguably this is
one of the signs that it wasn't the right trait.

# Prior art
[prior-art]: #prior-art

Most memory-safe programming languages have safe abstractions around raw
handles. Most often, they simply avoid exposing the raw handles altogether,
such as in [C#], [Java], and others. Making it `unsafe` to perform I/O through
a given raw handle would let safe Rust have the same guarantees as those
effectively provided by such languages.

There are several crates on crates.io providing owning and borrowing file
descriptor wrappers. The [io-lifetimes README.md's Prior Art section]
describes these and details how io-lifetimes' similarities and differences
with these existing crates in detail. At a high level, these existing crates
share the same basic concepts that io-lifetimes uses. All are built around
Rust's lifetime and ownership concepts, and confirm that these concepts
are a good fit for this problem.

Android has special APIs for detecting improper `close`s; see
rust-lang/rust#74860 for details. The motivation for these APIs also applies
to I/O safety here. Android's special APIs use dynamic checks, which enable
them to enforce rules across source language boundaries. The I/O safety
types and traits proposed here are only aiming to enforce rules within Rust
code, so they're able to use Rust's type system to enforce rules at
compile time rather than run time.

[io-lifetimes README.md's Prior Art section]: https://github.com/sunfishcode/io-lifetimes#prior-art
[C#]: https://docs.microsoft.com/en-us/dotnet/api/system.io.file?view=net-5.0
[Java]: https://docs.oracle.com/javase/7/docs/api/java/io/File.html?is-external=true

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Formalizing ownership

This RFC doesn't define a formal model for raw handle ownership and lifetimes.
The rules for raw handles in this RFC are vague about their identity. What does
it mean for a resource lifetime to be associated with a handle if the handle is
just an integer type? Do all integer types with the same value share that
association?

The Rust [reference] defines undefined behavior for memory in terms of
[LLVM's pointer aliasing rules]; I/O could conceivably need a similar concept
of handle aliasing rules. This doesn't seem necessary for present practical
needs, but it could be explored in the future.

[reference]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html

# Future possibilities
[future-possibilities]: #future-possibilities

Some possible future ideas that could build on this RFC include:

 - Clippy lints warning about common I/O-unsafe patterns.

 - A formal model of ownership for raw handles. One could even imagine
   extending Miri to catch "use after close" and "use of bogus computed handle"
   bugs.

 - A fine-grained capability-based security model for Rust, built on the fact
   that, with this new guarantee, the high-level wrappers around raw handles
   are unforgeable in safe Rust.

 - There are a few convenience features which can be implemented for types
   that implement `AsFd`, `Into<OwnedFd>`, and/or `From<OwnedFd>`:
     - A `from_into_fd` function which takes a `Into<OwnedFd>` and converts it
       into a `From<OwnedFd>`, allowing users to perform this common sequence
       in a single step.
     - A `as_filelike_view::<T>()` function returns a `View`, which contains a
       temporary instance of T constructed from the contained file descriptor,
       allowing users to "view" a raw file descriptor as a `File`, `TcpStream`,
       and so on.

 - Portability for simple use cases. Portability in this space isn't easy,
   since Windows has two different handle types while Unix has one. However,
   some use cases can treat `AsFd` and `AsHandle` similarly, while some other
   uses can treat `AsFd` and `AsSocket` similarly. In these two cases, trivial
   `Filelike` and `Socketlike` abstractions could allow code which works in
   this way to be generic over Unix and Windows.

   Similar portability abstractions could apply to `From<OwnedFd>` and
   `Into<OwnedFd>`.

# Thanks
[thanks]: #thanks

Thanks to Ralf Jung ([@RalfJung]) for leading me to my current understanding
of this topic, for encouraging and reviewing drafts of this RFC, and for
patiently answering my many questions!

[@RalfJung]: https://github.com/RalfJung
[`File`]: https://doc.rust-lang.org/stable/std/fs/struct.File.html
[`TcpStream`]: https://doc.rust-lang.org/stable/std/net/struct.TcpStream.html
[`FromRawFd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.FromRawFd.html
[`FromRawHandle`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.FromRawHandle.html
[`FromRawSocket`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.FromRawSocket.html
[`AsRawFd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.AsRawFd.html
[`AsRawHandle`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.AsRawHandle.html
[`AsRawSocket`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.AsRawSocket.html
[`IntoRawFd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.IntoRawFd.html
[`IntoRawHandle`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.IntoRawHandle.html
[`IntoRawSocket`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.IntoRawSocket.html
[`RawFd`]: https://doc.rust-lang.org/stable/std/os/unix/io/type.RawFd.html
[`RawHandle`]: https://doc.rust-lang.org/stable/std/os/windows/io/type.RawHandle.html
[`RawSocket`]: https://doc.rust-lang.org/stable/std/os/windows/io/type.RawSocket.html
[`AsRawFd::as_raw_fd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.AsRawFd.html#tymethod.as_raw_fd
[`FromRawFd::from_raw_fd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.FromRawFd.html#tymethod.from_raw_fd
[`from_raw_fd`]: https://doc.rust-lang.org/stable/std/os/unix/io/trait.FromRawFd.html#tymethod.from_raw_fd
[`from_raw_handle`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.FromRawHandle.html#tymethod.from_raw_handle
[`from_raw_socket`]: https://doc.rust-lang.org/stable/std/os/windows/io/trait.FromRawSocket.html#tymethod.from_raw_socket
[`std::mem::forget`]: https://doc.rust-lang.org/stable/std/mem/fn.forget.html
[`SockRef::from`]: https://docs.rs/socket2/0.4.0/socket2/struct.SockRef.html#method.from
[`unsafe_io::OwnsRaw`]: https://docs.rs/unsafe-io/0.6.2/unsafe_io/trait.OwnsRaw.html
[LLVM's pointer aliasing rules]: http://llvm.org/docs/LangRef.html#pointer-aliasing-rules
[`nix`]: https://crates.io/crates/nix
[`mio`]: https://crates.io/crates/mio
[`socket2`]: https://crates.io/crates/socket2
[`unsafe-io`]: https://crates.io/crates/unsafe-io
[`posish`]: https://crates.io/crates/posish
[rust-lang/rust#76969]: https://github.com/rust-lang/rust/pull/76969
[`memfd_create`]: https://man7.org/linux/man-pages/man2/memfd_create.2.html
