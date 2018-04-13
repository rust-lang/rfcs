- Feature Name: zero_page_optimization
- Start Date: 2018-04-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend the null pointer optimization to any value inside the zero page (which a
reference cannot have the value).

# Motivation
[motivation]: #motivation

Modern operating systems normally [traps null pointer access](https://en.wikipedia.org/wiki/Zero_page).
This means valid pointers will never take values inside the zero page, and we
can exploit this for ~12 bits of storage for secondary variants.

[Inside Rust std](https://github.com/rust-lang/rust/blob/ca26ef321c44358404ef788d315c4557eb015fb2/src/liballoc/heap.rs#L238),
we use a "dangling" pointer for ZST allocations; this involves a somewhat
verbose logic.

Outside std, we also see `futures-util`
[uses 1](https://github.com/rust-lang-nursery/futures-rs/blob/856fde847d4062f5d2af5d85d6640028297a10f1/futures-util/src/lock.rs#L157-L169)
as a special pointer value.

However, this is not something that is documented in the nomicon, neither it's
always true. For instance, microcontrollers without MMU doesn't implement such
guards at all, and `0` and `1` is a valid address where the entrypoint lies. See
[Cortex-M4](https://developer.arm.com/docs/ddi0439/latest/programmers-model/system-address-map)'s
design as one of such example.

Such crates should not assume anything regarding Rust ABI internals, but in the
case of this `BiLock`, we rely on compressing it into a usize so we can perform
atomic operations without a mutex. In practice, the entrypoint at `0` is
unlikely to be filled with Rust code but platform-specific bootstrap assembly.
Also, other factors like alignment also get involved so in practice we can't
collide the address. However, this RFC proposes a more logical and typed way
to code such things.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This change should be transparent for most users; the following description is
targeted at people dealing with FFI or unsafe.

The recently stabilized `NonNull` type will have more strict requirements:
the pointer must be not in the null page, and it must be valid to dereference.

`&T`, `&mut T`, `NonNull<T>` will have the same ranging semantics:
they will not take any value inside the zero page. We will optimize the layout
of an enumeration in a way similar to before, except that we will allow
discriminants of up to the zero page size (typically 4095).

Also, attempts to compress discriminants will be performed: which means, an
`Option<Option<&T>>` will be flattened internally, so its layout will be similar
to:

```rust
enum ... {
    NoneInner, // discriminant 0
    NoneOuter, // discriminant 1
    Some(&T)   // remainder
}
```

Note that here, we assign discriminants from inner to outer. This makes the
representation match when a reference is taken.

The exact behavior of this optimization should be documented upon implementation,
for unsafe coding usage.

To take advantage of zero page optimization, use `transmute` from and to usize.
This will cause compilation to fail if such optimization is not permitted on
the target.

An crate attribute `zero_page_size` will be exposed for configuring the exact
size of the zero page. This is mainly targeted at microcontroller runtimes.

An `zero_page_size` `#[cfg]` attribute will also be exposed, to code a fallback
instead of failing in cases like above.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We will add a target-specific default to determine the availability and size
of the zero page. The zero page range starts from 0, and must be at least one
byte so that old code relying on null pointer optimization will not break.

For the defined range, the compiler must ensure that no pointer of which value
is inside the range could be created safely. On microcontrollers, a dumb solution
would be creating a nop sled at the entrypoint.

This optimization only applies to pointer-like values (which can be dereferenced),
and `std::num::NonZero` keeps its current behavior.

The pointer internals will be also adopted to use this scheme: `Unique<T>` should
be refactored to use an enum internally.

# Drawbacks
[drawbacks]: #drawbacks

- This can create discrimination between platforms, although whether it's preferred
over undefined behavior is debatable.
- Compressing discriminant is not very straightforward.

# Rationale and alternatives
[alternatives]: #alternatives

## On the "null range"

- If we allow "none" to be set as the zero page range, it will make `Option<&T>`'s
layout Rust specific, which can't be used in FFI anymore. On microcontrollers
FFI should still be possible, so such breaking change isn't acceptable.
- We can also allow a very big value to use as "invalid page" range. However, this
may be incompatible with our current internals where `0` is considered `null`.

# Prior art
[prior-art]: #prior-art

Not applicable: Null pointer optimization is Rust specific, and this enhancement
is Rust specific too.

# Unresolved questions
[unresolved]: #unresolved-questions

- Can we suggest a better alternative than `transmute`? `transmute` is too
error prone despite we're trying to make the code more "safe".
