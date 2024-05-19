- Feature Name: `unsafe_extern`
- Start Date: 2023-05-23
- RFC PR: [rust-lang/rfcs#3484](https://github.com/rust-lang/rfcs/pull/3484)
- Tracking Issue: [rust-lang/rust#123743](https://github.com/rust-lang/rust/issues/123743)

# Summary
[summary]: #summary

It is unsafe to declare an `extern` block.  Starting in Rust 2024, all `extern` blocks must be marked as `unsafe`.  In all editions, items within `unsafe extern` blocks may be marked as safe to use.

# Motivation
[motivation]: #motivation

When we declare the signature of items within `extern` blocks, we are asserting to the compiler that these declarations are correct.  The compiler cannot itself verify these assertions.  If the signatures we declare are in fact not correct, then using these items may result in undefined behavior.  It's *unreasonable* to expect the *caller* (in the case of function items) to have to prove that the signature is valid.  Instead, it's the responsibility of the person writing the `extern` block to ensure the correctness of all signatures within.

Since this proof obligation must be discharged at the site of the `extern` block, and since this proof cannot be checked by the compiler, this implies that `extern` blocks are *unsafe*.  Correspondingly, we want to mark these blocks with the `unsafe` keyword and fire the `unsafe_code` lint for them.

By making clear where this proof obligation sits, we can now allow for items that can be soundly used directly from *safe* code to be declared within `unsafe extern` blocks.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust code can use functions and statics from foreign code.  The type signatures of these foreign items must be provided by the programmer in `extern` blocks.  These blocks must contain correct signatures to avoid undefined behavior.  The Rust compiler cannot check the correctness of the signatures in these blocks, so writing these blocks is *unsafe*.

An `extern` block may be placed anywhere a function declaration may appear.

- On editions >= 2024, you *must* write all `extern` blocks as `unsafe extern`.
- On editions < 2024, you *may* write `unsafe extern`, or you may write an `extern` block without the `unsafe` keyword.  Writing an `extern` block without the `unsafe` keyword is provided for compatibility only, and will eventually generate a warning.
- Use of `unsafe extern`, in all editions, fires the `unsafe_code` lint.

Within an `extern` block are zero or more declarations of external functions and/or external statics.  An extern function is declared with a `;` (semicolon) instead of a function body (similar to a method of a trait).  An extern static value is also declared with a `;` (semicolon) instead of an expression (similar to an associated const of a trait).  In both cases, the actual function body or value is provided by some external source.

Declarations within an `unsafe extern` block *may* annotate their signatures with either `safe` or `unsafe`.  If a signature within the block is not annotated, it is assumed to be `unsafe`.  The `safe` keyword is contextual and is currently allowed only within `extern` blocks.

If an `extern` block is used in an older edition without the `unsafe` keyword, item declarations *may not* specify `safe` or `unsafe`.  Code must update to `unsafe extern` to make `safe` item declarations.

```rust
unsafe extern {
    // sqrt (from libm) may be called with any `f64`
    pub safe fn sqrt(x: f64) -> f64;

    // strlen (from libc) requires a valid pointer,
    // so we mark it as being an unsafe fn
    pub unsafe fn strlen(p: *const c_char) -> usize;

    // this function doesn't say safe or unsafe, so it defaults to unsafe
    pub fn free(p: *mut core::ffi::c_void);

    pub safe static IMPORTANT_BYTES: [u8; 256];

    pub safe static LINES: SyncUnsafeCell<i32>;
}
```

Once unsafely declared, a `safe` item within an `unsafe extern` block may be used directly from safe Rust code.  The unsafe obligation of ensuring that the signature is correct is discharged by the block that declares the signature for the item.

When an item is declared as `unsafe`, as is usual in Rust, that means that the caller (or, in general, the user) may need to uphold certain unchecked obligations so as to prevent undefined behavior, and consequently that the item may only be used within an `unsafe` block.  However, the `extern` block (not the caller or other user) is still responsible for ensuring that the signature of that item is correct.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The grammar of the language is updated so that:

- Editions >= 2024 *must* prefix all `extern` blocks with `unsafe`.
- Editions < 2024 *should* prefix `extern` blocks with `unsafe`; this will eventually be a warn-by-default compatibility lint when `unsafe` is missing.

This RFC replaces the *["functions"][]* and *["statics"][]* sections in the [external blocks][] chapter of the Rust Reference with the following:

["functions"]: https://doc.rust-lang.org/nightly/reference/items/external-blocks.html#functions
["statics"]: https://doc.rust-lang.org/nightly/reference/items/external-blocks.html#statics
[external blocks]: https://doc.rust-lang.org/nightly/reference/items/external-blocks.html

### Functions

Functions within external blocks are declared in the same way as other Rust functions, with the exception that they must not have a body and are instead terminated by a semicolon.  Patterns are not allowed in parameters, only `IDENTIFIER` or `_` (underscore) may be used.  The function qualifiers `const`, `async`, and `extern` are not allowed.  If the function is unsafe to call, then the function should use the `unsafe` qualifier.  If the function is safe to call, then the function should use the `safe` qualifier (a contextual keyword).  Functions that are not qualified as `unsafe` or `safe` are assumed to be `unsafe`.

If the function signature declared in Rust is incompatible with the function signature as declared in the foreign code, the behavior of the resulting program may be undefined.

Functions within external blocks may be called by Rust code, just like functions defined in Rust.  The Rust compiler will automatically use the correct foreign ABI when making the call.

When coerced to a function pointer, a function declared in an extern block has type:

```rust
extern "abi" for<'l1, ..., 'lm> fn(A1, ..., An) -> R
```
where `'l1`, ..., `'lm` are its lifetime parameters, `A1`, ..., `An` are the declared types of its parameters and `R` is the declared return type.

### Statics

Statics within external blocks are declared in the same way as statics outside of external blocks, except that they do not have an expression initializing their value.  If the static is unsafe to access, then the static should use the `unsafe` qualifier.  If the static is safe to access (and immutable), then the static should use the `safe` qualifier (a contextual keyword).  Statics that are not qualified as `unsafe` or `safe` are assumed to be `unsafe`.

Extern statics may be either immutable or mutable just like statics outside of external blocks.  An immutable static must be initialized before any Rust code is executed.  It is not enough for the static to be initialized before Rust code reads from it.  A mutable extern static is always `unsafe` to access, the same as a Rust mutable static, and as such may not be marked with a `safe` qualifier.

# Drawbacks
[drawbacks]: #drawbacks

This change will induce some churn.  Hopefully, allowing people to safely call some foreign functions will make up for that.

# Alternatives
[alternatives]: #alternatives

## Don't prefix `extern` with `unsafe`

One could ask, why not allow each item within an `extern` block to be prefixed with either `safe` or `unsafe`, but do not prefix `extern` with `unsafe`?  E.g.:

```rust
extern {
    pub safe fn sqrt(x: f64) -> f64;
    pub unsafe fn strlen(p: *const c_char) -> usize;
}
```

Here's the problem with this.  The programmer is asserting that these signatures are correct, but this assertion cannot be checked by the compiler.  The human must simply get these correct, and if that person doesn't, then calling either of these functions, even the one marked `safe`, may result in undefined behavior.

In Rust, we use `unsafe { .. }` (and, as of [RFC 3325][], `unsafe(..)`) to indicate that what is enclosed must be proven correct by the programmer to avoid undefined behavior.  This RFC extends this pattern to `extern` blocks.

[RFC 3325]: https://github.com/rust-lang/rfcs/pull/3325

## Don't prefix `extern` with `unsafe` and support `unsafe` items only

One could ask, why not support only `unsafe` items within `extern` blocks and then don't require those blocks to be marked `unsafe`? E.g.:

```rust
extern {
    pub unsafe fn sqrt(x: f64) -> f64;
    pub unsafe fn strlen(p: *const c_char) -> usize;
}
```

One could argue that, since an `unsafe { .. }` block must be used to call either of these functions, that this is OK.

There are two problems with this.

One, we have to think about *who* is responsible for discharging the obligation of ensuring that these signatures are correct.  Is it the responsibility of a *caller* to these functions to ensure the signatures are correct?  That would seem unreasonable.  So even though the caller has to write `unsafe { .. }` to call these functions, this suggests that the `extern` *itself* should be somehow marked with or wrapped in `unsafe`.

Two, not allowing items to be marked as `safe` would remove one of the key tangible *benefits* that the changes in this RFC provide to users.  This would reduce the motivation to make this change at all.

## Prefix only `extern` with `safe` or `unsafe`

One could ask, why not prefix *only* `extern` with `safe` or `unsafe`?  E.g.:

```rust
safe extern {
    pub fn sqrt(x: f64) -> f64;
}
unsafe extern {
    pub fn strlen(p: *const c_char) -> usize;
}
```

The problem with this, as explained in the last two sections, is that the person who writes the `extern` block must discharge an unchecked obligation of proving that the signatures are correct.  This must be proven by the programmer even for the `sqrt` function.  One purpose of this RFC is to flag this obligation with `unsafe`.  This variation would fail to do that.

## Wrap `extern` in `unsafe { .. }`

Semantically, what we're trying to express is probably most precisely represented by syntax such as:

```rust
unsafe { extern {
    pub safe fn sqrt(x: f64) -> f64;
    pub unsafe fn strlen(p: *const c_char) -> usize;
}}
```

However, we currently don't support `unsafe { .. }` blocks at the item level, and the extra set of braces and indentation would seem unfortunate here.  One way to think of `unsafe extern { .. }` is exactly as above, but with the braces elided.

## Don't add the `safe` contextual keyword, flip the default

One could ask, why include the `safe` contextual keyword at all?  Why not just *assume* that within an `unsafe extern` block that items not marked as `unsafe` are in fact safe to call (as is true elsewhere in Rust)?  E.g.:

```rust
unsafe extern {
    pub fn sqrt(x: f64) -> f64; // Safe to call.
    pub unsafe fn strlen(p: *const c_char) -> usize;
}
```

This was in fact the original proposal.  The reason we did not end up adopting this was to reduce the churn that users would experience and to make the transition more incremental.

Consider that all `extern` blocks today look like this:

```rust
extern {
    pub fn sqrt(x: f64) -> f64; // Unsafe to call.
    pub fn strlen(p: *const c_char) -> usize; // Unsafe to call.
    // Many more items follow...
}
```

We want users to be able to adopt the new syntax by changing just one line, e.g.:

```rust
unsafe extern { // <--- We added `unsafe` here.
    pub fn sqrt(x: f64) -> f64; // Unsafe to call.
    pub fn strlen(p: *const c_char) -> usize; // Unsafe to call.
    // Many more items follow...
}
```

If we had made it so that writing `unsafe extern` flipped the default and made each item safe to call, then users would, upon making this change, have to simultaneously examine each item to determine whether it should be safe to call (or, at least, would have to conservatively add `unsafe` to each item).  We wanted to avoid this.

Still, the user gets immediate *benefit* out of this change, because the user can now *incrementally* mark items as safe to call, e.g.:

```rust
unsafe extern {
    pub safe fn sqrt(x: f64) -> f64; // <--- We added `safe` here.
    pub fn strlen(p: *const c_char) -> usize; // Unsafe to call.
    // Many more items follow...
}
```

We may or may not, in a later edition, decide to switch the default and thereby make the `safe` contextual keyword redundant.  Either way, adding the `safe` keyword makes the migration more straightforward while delivering value to users and better indicating where users must make a correctness assertion to the compiler.

## Don't add the `safe` contextual keyword, keep the default

One could ask, why not allow but not require items within an `unsafe extern` block to be prefixed with `unsafe`, but not support prefixing items with `safe`, and treat items not prefixed as `unsafe`?  E.g.:

```rust
unsafe extern {
    pub fn sqrt(x: f64) -> f64; // Unsafe to call.
    pub unsafe fn strlen(p: *const c_char) -> usize;
}
```

Doing this would eliminate one of the key tangible benefits of this RFC, which is allowing users to express that an item declared within an `unsafe extern` block is in fact sound to use directly in safe code.

While we could, in a later edition, perhaps flip the default to make items safe to call, we could only do that if enough code has already been migrated.  But in the interim, we'd be asking users to accept the churn of migrating to this syntax without receiving any of the benefits.  That seems a bit like a cyclic dependency, so we've chosen not to do that.

## Require all items to be marked as either `safe` or `unsafe`

One could ask, why not require all items within an `unsafe extern` block to be marked as either `safe` or `unsafe` rather than making this optional?  Or alternatively, one could ask, why not *only* allow items to be marked as `unsafe` and *require* that all items be marked `unsafe`?

As described in the last section, doing this would lead to a worse migration story for users, and so we chose not to do this.

## Wait until we switch to `safe { .. }` blocks

One could ask, why not wait to do this at all until we switch the language to use `safe { .. }` rather than `unsafe { .. }` blocks and then align this RFC with that?

The problem with this is that there is no current plan to make such a switch.  Waiting to improve the language on a possibility that may or may not happen -- and in any case, will not happen soon -- is usually not a good plan.

## Use `trusted` as the contextual keyword

One could ask, why not use `trusted` rather than `safe` as the contextual keyword?  E.g.:

```rust
unsafe extern {
    pub trusted fn sqrt(x: f64) -> f64; // Safe to call.
    pub unsafe fn strlen(p: *const c_char) -> usize;
}
```

The Rust language already has an accepted semantic for "safe" and "unsafe".  If we were to introduce a separated "trusted" concept, that would need to be part of a larger plan.  Such a plan does not yet exist in any concrete form, and it's not clear at this point whether any plan along these lines will succeed in gaining consensus.  Waiting to deliver value here on that possibility seems like a bad plan.

If we later decide, e.g., to replace all uses of `unsafe { .. }` with `trusted { .. }`, large amounts of code would need to be changed in that migration.  Changing from `safe fn` to `trusted fn` as part of that, as this RFC would require, doesn't seem that it would make that migration markedly more painful.

## Fire the `unsafe_code` lint for `extern` blocks also

This RFC specifies that the `unsafe_code` lint will fire for `unsafe extern` but not for `extern` blocks.  One could ask, why not fire this for `extern` blocks also?

The problem with doing this is that it may be very noisy on existing editions.  We're careful when expanding the meaning of existing lints to not create too much noise for existing code on existing editions, and doing this, at least immediately, may run afoul of this.

Of course, when migrating code to the *new* edition, people will be changing from `extern` to `unsafe extern`, and so if these people have both specifically turned up the severity of the `unsafe_code` lint (which, by default, is set to `allow`) and have `extern` blocks that now must be marked as `unsafe`, they will see this lint.  That is the intention of this change, as we're making clear that the person writing an `unsafe extern` block is responsible for proving that it is correct to ensure soundness, which makes this code *unsafe*.

# Questions and answers
[q-and-a]: #q-and-a

## Why do we want to mark `extern` blocks as `unsafe`?

In *safe* Rust, we want the compiler to *prove* that all code is *sound* and therefore cannot exhibit undefined behavior.  However, for some things, the compiler cannot complete this proof without help from the programmer.  When the programmer must make assertions that cannot be checked by the compiler to preserve soundness, we call this *unsafe* Rust.  We use the `unsafe` keyword to designate places where the programmer has this proof obligation.

In the past, `extern` blocks have been an exception to this.  Programmers are required to prove that these blocks are correct, and the compiler has no way of checking this, but we had yet not thought to write `unsafe` here.  This RFC closes that gap.

## Is adding this feature going to break people's existing code on existing editions?

No.  Rust has a stability guarantee that is outlined in [RFC 1122][].  Adding this feature does not break any existing code on existing editions when updating to newer versions of the Rust compiler.

[RFC 1122]: https://github.com/rust-lang/rfcs/pull/1122

## Will `extern` blocks not marked `unsafe extern` fire the `unsafe_code` lint?

No.  This RFC specifies that `unsafe extern` blocks will fire this lint.  There are no such blocks in the ecosystem today, so people who have `#![forbid(unsafe_code)]`  will only newly encounter this lint when switching a block from `extern` to `unsafe extern`.

## Does this RFC require all items in an `unsafe extern` block to be marked `safe` or `unsafe`?

No.  This RFC allows for items within an `unsafe extern` block to not be marked with either of `safe` or `unsafe`.  Items that are not marked in either way are assumed to be `unsafe`.

## What's the #46188 situation?

Currently, an `extern` block with incorrect signatures can result in a program exhibiting undefined behavior even if none of the items within that block are used by Rust code.  See, e.g., [#46188][].

Originally, the possibility of this undefined behavior was one of the motivations for this RFC.  However, it's possible that we may be able to resolve this in other ways, so we have redrafted this RFC to exclude this as a motivation.

The key motivation for this RFC is to make clear that the person writing an `extern` block is responsible for proving the correctness of the signatures within and that the compiler cannot check this proof.

[#46188]: https://github.com/rust-lang/rust/issues/46188

# Future possibilities
[future-possibilities]: #future-possibilities

## Interaction with extern types

If we were to later accept [RFC 3396][] ("Extern types v2"), that would introduce `type` items into `extern` blocks, and the interaction between those items, this RFC, and the `unsafe_code` lint would need to be addressed.

[RFC 3396]: https://github.com/rust-lang/rfcs/pull/3396
