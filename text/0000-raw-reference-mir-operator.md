- Feature Name: raw_reference_mir_operator
- Start Date: 2018-11-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a new primitive operator on the MIR level: `&[mut|const] raw <place>`
to create a raw pointer to the given place (this is not surface syntax, it is
just how MIR might be printed).  Desugar the surface syntax `&[mut] <place> as
*[mut|const] _` as well as coercions from references to raw pointers to use this
operator, instead of two MIR statements (first take normal reference, then
cast).

# Motivation
[motivation]: #motivation

Currently, if one wants to create a raw pointer pointing to something, one has
no choice but to create a reference and immediately cast it to a raw pointer.
The problem with this is that there are some invariants that we want to attach
to references, that have to *always hold*.  (This is not finally decided yet,
but true in practice because of annotations we emit to LLVM.  It is also the
next topic of discussion in the
[Unsafe Code Guidelines](https://github.com/rust-rfcs/unsafe-code-guidelines/).)
In particular, references must be aligned and dereferencable, even when they are
created and never used.

One consequence of these rules is that it becomes essentially impossible to
create a raw pointer pointing to an unaligned struct field: `&packed.field as
*const _` creates an immediate unaligned reference, triggering undefined
behavior because it is not aligned.  Similarly, `&(*raw).field as *const _` is
not just computing an offset of the raw pointer `raw`, it also asserts that the
intermediate shared reference is aligned and dereferencable.  In both cases,
that is likely not what the author of the code intended.

To fix this, we propose to introduce a new primitive operation on the MIR level
that, in a single MIR statement, creates a raw pointer to a given place.  No
intermediate reference exists, so no invariants have to be adhered to.  We also
add a lint for cases that seem like the programmer wanted a raw reference, not a
safe one, but did not use the right syntax.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When working with unaligned or potentially dangling pointers, it is crucial that
you always use raw pointers and not references: References come with guarantees
that the compiler assumes are always upheld, and these guarantees include proper
alignment and not being dangling.  Importantly, these guarantees must be
maintained even when the reference is created and never used!  The following is
UB:

```rust
#[repr(packed)]
struct Packed {
    pad: u8,
    field: u16,
}
let packed = Packed { pad: 0, field: 0 };
let x = unsafe { &packed.field }; // `x` is not aligned -> undefined behavior
```

There is no situation in which the above code is correct, and hence it is a hard
error to write this.  This applies to most ways of creating a reference, i.e.,
all of the following are UB if `X` is not properly aligned and dereferencable:

```rust
fn foo() -> &T {
  &X
}

fn bar(x: &T) {}
bar(&X); // this is UB at the call site, not in `bar`

let &x = &X; // this is actually dereferencing the pointer, certainly UB
let _ = &X; // throwing away the value immediately changes nothing
&X; // different syntax for the same thing

let x = &X as &T as *const T; // this is casting to raw "too late"
```

The only way to create a pointer to an unaligned or dangling location without
triggering undefined behavior is to *immediately* turn it into a raw pointer
using an explicit cast or an implicit coercion.  All of the following are valid:

```rust
let packed_cast = unsafe { &packed.field as *const _ };
let packed_coercion: *const _ = unsafe { &packed.field };
let null_cast: *const _ = unsafe { &*ptr::null() } as *const _;
let null_coercion: *const _ = unsafe { &*ptr::null() };
```

The intention is to cover all cases where a reference, just created, is
immediately explicitly used as a value of raw pointer type.

These two operations (taking a reference, casting/coercing to a raw pointer) are
actually considered a single operation happening in one step, and hence the
invariants incurred by references do not come into play.

Notice that this only applies if no automatic call to `deref` or `deref_mut` got
inserted: those are regular function calls taking a reference, so in that case a
reference is created and it must satisfy the usual guarantees.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When translating HIR to MIR, we recognize `&[mut] <place> as *[mut|const] ?T`
(where `?T` can be any type, also a partial one like `_`) as well as coercions
from `&[mut] <place>` to a raw pointer type as a special pattern and turn them
into a single MIR `Rvalue` that takes the address and produces it as a raw
pointer -- a "take raw reference" operation.  We do this *after* auto-deref,
meaning this pattern does not apply when a call to `deref` or `deref_mut` got
inserted.  We also use this new `Rvalue` to translate `x as *[mut|const] ?T`;
before this RFC such code gets translated to MIR as a reborrow followed by a
cast.  Once this is done, `Misc` casts from reference to raw pointers can be
removed from MIR, they are no longer needed.

This new `Rvalue` might be a variant of the existing `Ref` operation (say, a
boolean flag for whether this is raw), or a new `Rvalue` variant.  The borrow
checker should do the usual checks on `<place>`, but can just ignore the result
of this operation and the newly created "reference" can have any lifetime.
(Before this RFC this will be some form of unbounded inference variable because
the only use is a cast-to-raw, the new "raw reference" operation can have the
same behavior.)  When translating MIR to LLVM, nothing special has to happen as
references and raw pointers have the same LLVM type anyway; the new operation
behaves like `Ref`.

When interpreting MIR in the Miri engine, the engine will recognize that the
value produced by this `Rvalue` has raw pointer type, and hence needs not
satisfy any special invariants.

When doing unsafety checking, we make references to packed fields that do *not*
use this new "raw reference" operation a *hard error even in unsafe blocks*
(after a transition period).  There is no situation in which this code is okay;
it creates a reference that violates basic invariants.  Taking a raw reference
to a packed field, on the other hand, is a safe operation as the raw pointer
comes with no special promises.  "Unsafety checking" is thus not even a good
term for this, maybe it should be a special pass dedicated to packed fields
traversing MIR, or this can happen when lowering HIR to MIR.  This check has
nothing to do with whether we are in an unsafe block or not.

Moreover, to prevent programmers from accidentally creating a safe reference
when they did not want to, we add a lint that identifies situations where the
programmer likely wants a raw reference, and suggest an explicit cast in that
case.  One possible heuristic here would be: If a safe reference (shared or
mutable) is only ever used to create raw pointers, then likely it could be a raw
pointer to begin with.  The details of this are best worked out in the
implementation phase of this RFC.

# Drawbacks
[drawbacks]: #drawbacks

It might be surprising that the following two pieces of code are not equivalent:
```rust
// Variant 1
let x = unsafe { &packed.field }; // Undefined behavior!
let x = x as *const _;
// Variant 2
let x = unsafe { &packed.field as *const _ };
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is a compromise: I see no reasonable way to translate the first variant
shown in the "Drawbacks" section to a raw reference operation, and the second
variant is so common that we likely do not want to rule it out.  Hence the
proposal to make them not equivalent.

One alternative to introducing a new primitive operation might be to somehow
exempt "references immediately cast to a raw pointer" from the invariant.
However, we believe that the semantics of a MIR program, including whether it
has undefined behavior, should be deducible by executing it one step at a time.
Given that, it is unclear how a semantics that "lazily" checks references should
work, and how it could be compatible with the annotations we emit for LLVM.

Instead of compiling `&[mut] <place> as *[mut|const] ?T` to a raw reference
operation, we could introduce new surface syntax and keep the existing HIR->MIR
lowering the way it is.  However, that would make lots of carefully written
existing code dealing with packed structs have undefined behavior.  (There is
likely also lots of code that forgets to cast to a raw pointer, but I see no way
to make that legal -- and the proposal would make such uses a hard error in the
long term, so we should catch many of these bugs.)  Also, no good proposal for a
surface syntax has been made yet -- and if one comes up later, this proposal is
forwards-compatible with also having explicit syntax for taking a raw reference
(and deprecating the safe-ref-then-cast way of writing this operation).

We could be using the new operator in more cases, e.g. instead of having a smart
lint that tells people to insert casts, we could use that same analysis to infer
when to use a raw reference.  This would make more code use raw references, thus
making more code defined.  However, if someone *relies* on this behavior there
is a danger of accidentally adding a non-raw-ptr use to a reference, which would
then rather subtly make the program have UB.  That's why we proposed this as a
lint instead.

# Prior art
[prior-art]: #prior-art

I am not aware of another language with both comparatively strong invariants for
its reference types, and raw pointers.  The need for taking a raw reference only
arise because of Rust having both of these features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

We could have different rules for when to take a raw reference (as opposed to a
safe one).

Does the operator creating a raw pointer allow creating pointers that are not
dereferencable (with the size determined by `mem::size_of_val`)?  It might turn
out to be useful to make dereferencability not part of the validity invariant,
but part of the alias model, so this is a separate question from whether the
pointer is aligned and non-NULL.  Notice that we use `getelementptr inbounds`
for field access, so we would require some amount of dereferencability anyway
(or we could change codegen to not emit `inbounds` when creating a raw
reference, but that might adversely affect performance).

The interaction with auto-deref is a bit unfortunate.  Maybe we can have a lint
to detect what seems to be unwanted cases of auto-deref -- namely, terms that
look like `&[mut] <place> as *[mut|const] ?T` in the surface syntax but had a
method call inserted, thus manifesting a reference (with the associated
guarantees) where none might be expected.
