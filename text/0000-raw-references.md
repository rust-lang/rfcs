- Feature Name: raw_reference_tracking
- Start Date: 2019-01-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Formalizes an internal mechanism to track references on which the compiler
should not enforce reference properties. This complements MIR level changes
through which taking the address of a subobject pointed to by a pointer does not
accidentally require or guarantee reference properties on those pointers.

# Motivation
[motivation]: #motivation

In current Rust semantics to obtain a pointer to some place, one creates a
reference and casts it to a pointer. Since we strictly speaking attach even to
this temporary reference some additional invariants, this may not be currently
sound in MIR.  These invariants (among them alignedness and dereferencability)
are not required for pointers. Since any inspection of a type's fields involves
an intermediate reference to the extracted field, it is impossible to soundly
retrieve a pointer to such.

A [similarly motivated RFC](https://github.com/rust-lang/rfcs/pull/2582) exists
that tries to approach this problem by adding a MIR operation that performs a
direct pointer to pointer-to-field conversion in a few defined cases. This
leaves open the question of a formalized approach to the problem and implements
a mostly syntactical solution, rather than a type-level one. While exposing such
properties through the type system would not be backwards compatible, it could
still be possible to attach such properties internally. Through this system, we
can also provide better warnings and give the necessary foundation to properly
discuss extended guarantees.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Each reference (`&` or `&mut`) will have an internal–that is, not observable by
rust code–tracking state named `raw`. While a reference is `raw`, it will be
represented as a pointer in syntax-to-MIR lowering. The act of unactivating the
raw status of a reference will be called `settling` for the purpose of this
document, how this can happen will be discussed below. When a reference with
active `raw` status is converted to a pointer, this will be called a raw pointer
cast and would be a no-op in MIR.

```
// The result of this borrow is a raw reference.
let x = unsafe { &packed.field as *const _ };
```

In short, a reference will keep its raw status as long as its pointed-to content
is not accessed. Since the tracking of references should stay function local,
passing a reference as an argument to another function or returning it will
unset the `raw` status. Note that raw references can only originate from unsafe
code.  Thus, when a raw reference is cast to a pointer in safe code, this is an
indicator for potentially incorrect usage of unsafe and a warning could be
emitted.

Since this solution works at type-level, we keep the composability properties
that code relies on. In particular, we can decompose expressions into
subexpressions:

```
let x = unsafe {
    // Desugared version. `raw` is reference with `raw` status.
    let raw = &(*packed).field;
    // Cast with active raw status, raw pointer cast.
    raw as *const _
};
```

This also propagates out of unsafe blocks, even though a raw reference from a
pointer can only be created within an unsafe block. This proposal intends not to
stabilize the guarantee of the following code not invoking undefined behaviour
but focusses on nevertheless not exploiting it in MIR.

```
let x = unsafe { &packed.field };
let x = x as *const _;
```

Since this is the only use of `x`, and `x` is never settle but originated in an
unsafe block, this is a strong code smell. Specifically, it would be strictly
safer for this reference to never leave the unsafe code block. By tracking the
origin of a raw reference, a lint could be emitted to suggest moving the pointer
cast inside.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `raw` status of a reference can only originate from a borrow expression. The
reference result of a borrow expression `& (*t).field_path` will have the `raw`
status set when at least one of these is true:

* `t` has pointer type.
* `t` is itself a raw reference.
* `field_path` contains fields of a union or packed struct.

Additionally, the raw status of a reference is conserved across move and copy.
When a reference is used in any expression except `Copy`, `Move` or as the
pointer in a borrow expression from which another raw reference originates, then
it it settled. This implies that the analysis is local, as any return settles
the reference and no returned reference is raw.

On the MIR-level, we represent a reference with `raw` status as a pointer, and
convert it to an actual reference only when it is settled. This requires a MIR
operation that can borrow a place as a pointer directly. Such an operation has
been proposed and is currently in merge period already.

# Drawbacks
[drawbacks]: #drawbacks

This proposal complicates reasoning about unsafe code. It is no longer clear
from a single value expresion alone whether `& (*place)` will be enforced as
reference type, or simply treated like a pointer, even though we need not make
such guarantees to stable code yet.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative would be to create dedicated new surface level syntax that only
works on raw pointers to create another raw pointer using a place-like
expression. This design is preferable over a new syntax for creating temporary
raw references for some reasons:

Old code that relied on `&packed.field as *const_` and incorrectly from lack of
explicit other concepts inferred from this that any temporary reference
creation eventually cast to pointer is defined should be considered. This
reasoning would gain an explicit approval as long as the type is never
explicitly cast to/ascribed with/used as a reference.

A new surface level syntax would require deeper changes. Most uncomfortably, it
could require changing type resolution by adding new types. This would very
unlikely be backwards compatible while offering all necessary guarantees to
current code. In contrast, the mechanisms proposed here are a middle ground that
achieves some of the definedness guarantees internally without making them
stable. But it also enables lints that could help detect and eliminate mistakes
that could become strictly undefined if the internal mechanisms were reverted
again.

# Prior art
[prior-art]: #prior-art

An [alternate proposal](https://github.com/rust-lang/rfcs/pull/2582) addresses
the same underlying concern als with MIR changes. These are not exclusive and
MIR-only changes are generally preferable to surface language changes. This
proposal tries to make these changes more consistent at both levels. 

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Settling a raw reference is the one semantic case that creates a reference
'out of thin air', through `unsafe` blocks. For the sake of a programmatic
undefined behaviour sanitizer, the settling of raw references would thus be a
primary injection point. This opportunity could make it worth creating a
separate instruction rather than using `&(*ptr)`.

# Future possibilities
[future-possibilities]: #future-possibilities

Unsafe code that only performs reborrowing to raw references and casts all raw
references to pointers before they are settled does not actually execute any
code that must be specified as unsafe. Getting a pointer to a field from a
pointer to the containing struct should not, for example. Another example would
be a statement that gets the pointer to a field of a packed struct from a
reference to that struct. Since this internally only translates to safe code,
there is no intrinsic reason why it requires `unsafe`. However, the path+borrow
expression of such code currently requires the use of `unsafe`.

```
let field = &(*foo).bar as *const _;
let field = &packed.field as *mut _;
```

The notion of `raw` references could be extended and brought into language level
by making such code safe as long as it does not settle any raw references. This
could improve ergonomics by removing unecessary `unsafe` code blocks, and
further strengthen the justification requirements for the usage of `unsafe`.
Such a concept could however require a stronger foundation than this proposal
alone.

