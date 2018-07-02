- Feature Name: string_reflection
- Start Date: 2017-12-03
- RFC PR: 
- Rust Issue: https://github.com/rust-lang/rust/issues/46261

# Summary
[summary]: #summary

Add link-time hooks to libcore that would allow the `Debug` implementation
for trait object types `Any` and `Any + Send` to:
- type-check against the runtime's implementation of `String`,
  if one is available;
- get the owned string's content if the type matches.

# Motivation
[motivation]: #motivation

The trait `Any` is Rust's way to represent arbitrary values (of `'static`
types) while enabling runtime type checking and downcasting. One of the most
prominent uses of this trait, with a `Box<Any>` object pointer, is to convey
the value of the `panic!` invocation retrieved from a panicked thread.
The `Box<Any>` is returned by `std::thread::JoinHandle::join()` in the
`Err` variant. Calling code that does not expect the child thread to panic
often uses the `unwrap()` method of the `Result` value, so that, if the child
thread has panicked, the calling thread would panic in turn. The generic
implementation of `Result::unwrap()` uses the `Debug` implementation of the
`Err`'s value for content of its own panic message. However, the current
`Debug` implementation for `Any` trait objects [does not perform][rfc-issue]
any reflection on the value, therefore disrupting propagation of the
originating panic's message even if it is a string.

This leaves the panic hook as the only way to inform the user about the cause
of the originating panic. The process-global panic hook can be overridden
for unrelated reasons, leaving potential for unintended loss of information
on errors within the program.

In other uses, it would also be nice and informative to have the `Debug` impl
output the incidental string value under an `Any` trait object, while not
behaving differently for owned and static strings as long as they are
represented with their standard Rust types.

The problem with implementing it in this way is that a consistent
implementation would have to include a check for the `String` type,
which is not known to libcore where the implementation is defined.

[rfc-issue]: https://github.com/rust-lang/rfcs/issues/1389

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Debug` trait implementation for trait object type `Any` (and for
`Any + Send`) accommodates a special case: if the actual type of the value
is `&'static str` or `String` (temporary references or unsized slices can't
be under `Any`), the string content is formatted.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The main issue to solve here is how to make the impl code in libcore
recognize the type of an `Any` object value as `String` and get a peek
into its content, even as this allocating type is defined above libcore in the
dependency graph.

## String reflection protocol
[api]: #string-reflection-protocol

Libcore implements the dynamic type check for, and downcasting to the content
of, the owning string type if such exists in the linked runtime, using a
couple of external symbols that are linked with the implementation crate
by the compiler, defined approximately as:

```rust
extern {
    fn __rust_type_is_string(id: TypeId) -> bool;
    fn __rust_downcast_string_as_str<'a>(p: *const Any) -> &'a str;
}
```

The exact API of these symbols is considered **unstable**.
The formulation above ignores any representation issues with passing
`TypeId` and fat pointers though the ABI; if necessary, the value types
can be despecified to raw repr-friendly forms.

## String reflection attributes

Similarly to the approach taken with [custom allocators][allocator-attributes]
and the [panic runtime][panic-attributes], two new
**unstable** crate attributes will be added to the compiler:

* `#![needs_string_reflection]` - indicates that this crate requires the
  string reflection protocol described [above][api] to link correctly.
  This is intended to be only attached to libcore.
* `#![string_reflection]` - indicates that this crate has an implementation
  of the string reflection protocol.

[allocator-attributes]: https://github.com/rust-lang/rfcs/blob/master/text/1183-swap-out-jemalloc.md#new-attributes
[panic-attributes]: https://github.com/rust-lang/rfcs/blob/master/text/1513-less-unwinding.md#panic-attributes

The compiler will check that exactly one crate in the linkage DAG of a
complete artifact, such as an executable or a dylib crate, is tagged with
the `#![string_reflection]` attribute.

## Crates providing (or stubbing out) string reflection

In the default linkage, the crate providing string reflection will be
liballoc, the crate that defines `String`. It's hard to imagine a need for
alternative `String` implementations in the Rust ecosystem, so this RFC will
consider only one other case: no allocated strings at all. For this case,
a stub crate tentatively named `no_string` will be provided in the standard
distribution. The reflection protocol implementation in this crate will
never indicate a type as `String` for libcore.

# Drawbacks
[drawbacks]: #drawbacks

* To borrow a quote from the panic runtime RFC:
  This represents a proliferation of the `#![needs_foo]` and `#![foo]` style
  system that allocators have begun.

* Code may exist that relies on the `Debug` impl for `Any` to output just
  "Any", so that the change could result in breakage or information leaks.

# Rationale and alternatives
[alternatives]: #alternatives

The current behavior with unwrapping `thread::Result` is not a
world-breaker, but it leaves it to the panic hook to provide
information on the cause of a cascading panic. The design of panic hooks is
too restricted to provide non-global information retrieval facilities in
complex thread failure isolation setups and libraries.

Improvement of the `Debug` impl will not require any changes in usage,
except where the invoking code makes assumptions about the _output_.

An extension trait could be defined and blanket-implemented in libstd for
`Result<T, Box<Any + Send>>`, providing a variant method to `unwrap()` with
introspection into the incidental string `Err`.
The trait would need to be explicitly imported unless also added to
`std::prelude`, and it would be less discoverable than the inherent and
commonly used `unwrap()`.
Adding a workaround trait that would be only justified by the details of
crate organization below `std` seems odd.

# Unresolved questions
[unresolved]: #unresolved-questions

Should any compiler flags or crate attributes implicitly lead to linking
`no_string` instead of liballoc?
