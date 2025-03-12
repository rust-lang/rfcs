- Feature Name: `raw_ref_op`
- Start Date: 2018-11-01
- RFC PR: [rust-lang/rfcs#2582](https://github.com/rust-lang/rfcs/pull/2582)
- Rust Issue: [rust-lang/rust#64490](https://github.com/rust-lang/rust/issues/64490)

# Summary
[summary]: #summary

Introduce new variants of the `&` operator: `&raw mut <place>` to create a `*mut <T>`, and `&raw const <place>` to create a `*const <T>`.
This creates a raw pointer directly, as opposed to the already existing `&mut <place> as *mut _`/`&<place> as *const _`, which create a temporary reference and then cast that to a raw pointer.
As a consequence, the existing expressions `<term> as *mut <T>` and `<term> as *const <T>` where `<term>` has reference type are equivalent to `&raw mut *<term>` and `&raw const *<term>`, respectively.
Moreover, emit a lint for existing code that could use the new operator.

# Motivation
[motivation]: #motivation

Currently, if one wants to create a raw pointer pointing to something, one has no choice but to create a reference and immediately cast it to a raw pointer.
The problem with this is that there are some invariants that we want to attach to references, that have to *always hold*.
The details of this are not finally decided yet, but true in practice because of annotations we emit to LLVM.
It is also the next topic of discussion in the [Unsafe Code Guidelines](https://github.com/rust-rfcs/unsafe-code-guidelines/).
In particular, references must be aligned and dereferenceable, even when they are created and never used.

One consequence of these rules is that it becomes essentially impossible to create a raw pointer pointing to an unaligned struct field:
`&packed.field as *const _` creates an intermediate unaligned reference, triggering undefined behavior because it is not aligned.
Instead, code currently has to copy values around to aligned locations if pointers need to be passed, e.g., to FFI, as in:

```rust
#[derive(Default)] struct A(u8, i32);

let mut a: A = Default::default();
let mut local = a.1; // copy struct field to stack
unsafe { ffi_mod(&mut local as *mut _) }; // pass pointer to local to FFI
a.1 = local; // copy local to struct back
```

If one wants to avoid creating a reference to uninitialized data (which might or might not become part of the invariant that must be always upheld), it is also currently not possible to create a raw pointer to a field of an uninitialized struct:
again, `&mut uninit.field as *mut _` would create an intermediate reference to uninitialized data.

Another issue people sometimes run into is computing the address/offset of a field without asserting that there is any memory allocated there.
This actually has two problems; first of all creating a reference asserts that the memory it points to is allocated, and secondly the offset computation is performed using `getelementptr inbounds`, meaning that the result of the computation is `poison` if it is not in-bounds of the allocation it started in.
This RFC just solves the first problem, but it also provides an avenue for the second (see "Future possibilities").

To avoid making too many assumptions by creating a reference, this RFC proposes to introduce a new primitive operation that directly creates a raw pointer to a given place.
No intermediate reference exists, so no invariants have to be adhered to: the pointer may be unaligned and/or dangling.
We also add a lint for cases that seem like the programmer unnecessarily created an intermediate reference, suggesting they reduce the assumptions their code is making by creating a raw pointer instead.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When working with unaligned or potentially dangling pointers, it is crucial that you always use raw pointers and not references:
references come with guarantees that the compiler assumes are always upheld, and these guarantees include proper alignment and not being dangling.
Importantly, these guarantees must be maintained even when the reference is created and never used!
The following is UB:

```rust
#[repr(packed)]
struct Packed {
    pad: u8,
    field: u16,
}
let packed = Packed { pad: 0, field: 0 };
let x = unsafe { &packed.field }; // `x` is not aligned -> undefined behavior
```

There is no situation in which the above code is correct, and hence it is a hard error to write this (after a transition period).
This applies to most ways of creating a reference, i.e., all of the following are UB if `X` is not properly aligned and dereferenceable:

```rust
fn foo() -> &T {
  &X
}

fn bar(x: &T) {}
bar(&X); // this is UB at the call site, not in `bar`

let &x = &X; // this is actually dereferencing the pointer, certainly UB
let _ = &X; // throwing away the value immediately changes nothing
&X; // different syntax for the same thing

let x = &X as *const T; // this is casting to raw but "too late", an intermediate reference has been created
```

The only way to create a pointer to an unaligned or dangling location without triggering undefined behavior is to use `&raw`, which creates a raw pointer without an intermediate reference.
The following is valid:

```rust
let packed_cast = &raw const packed.field;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Rust contains two operators that perform place-to-value conversion (matching `&` in C): one to create a reference (with some given mutability) and one to create a raw pointer (with some given mutability).
In the MIR, this is reflected as either a distinct `Rvalue` or a flag on the existing `Ref` variant.
Lowering to MIR should *not* insert an implicit reborrow of `<place>` in `&raw mut <place>`; that reborrow would assert validity and thus defeat the entire point.
The borrow checker should do the usual checks on the place used in `&raw`, but can just ignore the result of this operation and the newly created "reference" can have any lifetime.
When translating MIR to LLVM, nothing special has to happen as references and raw pointers have the same LLVM type anyway; the new operation behaves like `Ref`.
When interpreting MIR in the Miri engine, the engine will know not to enforce any invariants on the raw pointer created by `&raw`.

Moreover, to prevent programmers from accidentally creating a safe reference when they did not want to, we add a lint that identifies situations where the programmer likely wants a raw reference, and suggest an explicit cast in that case.
One possible heuristic here would be: If a safe reference (shared or mutable) is only ever used to create raw pointers, then likely it could be a raw pointer to begin with.
The details of this are best worked out in the implementation phase of this RFC.
The lint should, at the very least, fire for the cases covered by the [syntactic sugar extension][future-possibilities], and it should fire when the factor that prevents this matching the sugar is just a redundant block, such as `{ &mut <place> } as *mut ?T`.

# Drawbacks
[drawbacks]: #drawbacks

This introduces new clauses into our grammar for a niche operation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One alternative to introducing a new primitive operation might be to somehow exempt "references immediately cast to a raw pointer" from the invariant.
(Basically, a "dynamic" version of the static analysis performed by the lint.)
However, I believe that the semantics of a MIR program, including whether it as undefined behavior, should be deducible by executing it one step at a time.
Given that, it is unclear how a semantics that "lazily" checks references should work, and how it could be compatible with the annotations we emit for LLVM.

As an alternative to `&raw const <place>`, one might want to use `&raw <place>` for better symmetry with shared references.
However, this introduces ambiguities into the parser because `raw` is not a keyword.
For further details, see discussion [here][syntax-1] and [here][syntax-2] and [here][syntax-3].

[syntax-1]: https://github.com/rust-lang/rfcs/pull/2582#issuecomment-465519395
[syntax-2]: https://github.com/rust-lang/rfcs/pull/2582#issuecomment-483439054
[syntax-3]: https://github.com/rust-lang/rfcs/pull/2582#issuecomment-489468105


# Prior art
[prior-art]: #prior-art

I am not aware of another language with both comparatively strong invariants for its reference types, and raw pointers.
The need for taking a raw reference only arise because of Rust having both of these features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Maybe the lint should also cover cases that look like `&[mut] <place> as *[mut|const] ?T` in the surface syntax but had a method call inserted, thus manifesting a reference (with the associated guarantees).
The lint as described would not fire because the reference actually gets used as such (being passed to `deref`).
However, what would the lint suggest to do instead?
There just is no way to write this code without creating a reference.

# Future possibilities
[future-possibilities]: #future-possibilities

## "Syntactic sugar" extension

We could treat `&mut <place> as *mut _`/`&<place> as *const _` as if they had been written with `&raw` to avoid creating temporary references when that was likely not the intention.
We could also do this when `&mut <place>`/`& <place>` is used in a coercion site and gets coerced to a raw pointer.

```rust
let x = &X as *const T; // this is fine now
let x: *const T; // this is fine if we also apply the "sugar" for coercions
let x = &X as &T as *const T; // this is casting to raw but "too late" even if we adapt [SUGAR]
let x = { &X } as *const T; // this is likely also too late (but should be covered by the lint)
let x: *const T = if b { &X } else { &Y }; // this is likely also too late (and hopefully covered by the lint)
```

Notice that this only applies if no automatic call to `deref` or `deref_mut` got inserted:
those are regular function calls taking a reference, so in that case a reference is created and it must satisfy the usual guarantees.

The point of this to keep existing code working and to provide a way for projects to adjust to these rules before stabilization.
Another good reason for this extension is that code could be adjusted without having to drop support for old Rust versions.

However, it might be surprising that the following two pieces of code are not equivalent:

```rust
// Variant 1
let x = unsafe { &packed.field }; // Undefined behavior!
let x = x as *const _;
// Variant 2
let x = unsafe { &packed.field as *const _ }; // good code
```

This is at least partially mitigated by the fact that the lint should fire in variant 1.

Another problem is that if `as` ever becomes an operation that can be overloaded, the behavior of `&packed.field as *const _` can *not* be obtained by dispatching to the overloaded `as` operator.
Calling that method would assert validity of the reference.

In the future, if Rust's type ascriptions end up performing coercions, those coercions should trigger the raw reference operator just like other coercions do.
So `&packed.field: *const _` would be `&raw const packed.field`.
If Rust ever gets type ascriptions with coercions for binders, likewise these coercions would be subject to these rules in cases like `match &packed.field { x: *const _ => x }`.

## Encouraging / requiring `&raw` in situations where references are often/definitely incorrect

We could make references to packed fields that do *not* use this new "raw reference" operation a *hard error even in unsafe blocks* (after a transition period).
There is no situation in which this code is okay; it creates a reference that violates basic invariants.
Taking a raw reference to a packed field, on the other hand, is a safe operation as the raw pointer comes with no special promises.

It has been suggested to [remove `static mut`][static-mut] because it is too easy to accidentally create references with lifetime `'static`.
With `&raw` we could instead restrict `static mut` to only allow taking raw pointers (`&raw [mut|const] STATIC`) and entirely disallow creating references (`&[mut] STATIC`) even in safe code (in a future edition, likely; with lints in older editions).

## Other

**Lowering of casts.** Currently, `mut_ref as *mut _` has a reborrow inserted, i.e., it gets lowered to `&mut *mut_ref as *mut _`.
It seems like a good idea to lower this to `&raw mut *mut_ref` instead to avoid any effects the reborrow might have in terms of permitted aliasing.
This has the side-effect of being able to entirely remove reference-to-pointer-*casts* from the MIR; that conversion would be done by a "raw reborrow" instead (which is consistent with the pointer-to-reference situation).

**`offsetof` woes.** As mentioned above, expressions such as `&raw mut x.field` still trigger more UB than might be expected---as witnessed by a [couple of attempts found in the wild of people implementing `offsetof`][offset-of] with something like:

```rust
let x: *mut Struct = NonNull::dangling().as_ptr();
let field: *mut Field = &mut (*x).field;
```

The lint as described in this RFC would nudge people to instead write

```rust
let x: *mut Struct = NonNull::dangling().as_ptr();
let field: *mut Field = &raw mut (*x).field;
```

which is better, but still UB: we emit a `getelementptr inbounds` for the `.field` offset computation.
It might be a good idea to just not do that -- we know that references are fine, but we could decide that when raw pointers are involved that might be dangling, we do not want to assert anything from just the fact that an offset is being computed.
However, there are concerns that a plain `getelementptr` will not be sufficiently optimized because it also permits arithmetic that wraps around the end of the address space.
LLVM currently does not support a `getelementptr nowrap` that disallows wrapping but permits cross-allocation arithmetic, but if that could be added, using it for raw pointers could save us from having to talk about the "no outofbounds arithmetic" rule in the semantics of field access (the UB triggered by creating dangling references would be enough).
If people just hear "`&raw` means my pointer can be dangling" they might think the second version above is actually okay, forgetting that the field access itself has its own subtle rule; getting rid of that rule would remove one foot-gun for unsafe code authors to worry about.

[static-mut]: https://github.com/rust-lang/rust/issues/53639
[offset-of]: https://github.com/rust-lang/rfcs/pull/2582#issuecomment-467629986
