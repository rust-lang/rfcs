- Feature Name: anon_lifetime
- Start Date: 2015-06-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow using an undeclared '_ wherever an explicit lifetime can be used, but is
optional, such as function argument/return types and any path inside a function.

# Motivation

During the addition of 'tcx in the compiler (denoting the lifetime of the
arenas used to allocate all the type-system information in the compiler,
via the type context aka `ty::ctxt` aka `tcx`), I had attempted to replace
`&'a ty::ctxt<'tcx>` with a clean `TyCx<'a, 'tcx>` wrapper.
When both `'a` and `'tcx` are either present or missing, it works very well,
but the common case is `&ty::ctxt<'tcx>`, which cannot be represented by
`TyCx<'a, 'tcx>` without adding a new explicit lifetime parameter (`'a`)
to the appropriate function or impl block.

I've recently done this as part of rust-lang/rust#26575, but it wasn't easy
or pretty and any attempts at adding another lifetime to `TyCx` (necessary
for splitting type contexts for performance and possibly parallelism in the
future) would result in more noise at almost every use site of `TyCx`.

# Detailed design

In `resolve_lifetime`: if the lifetime to be resolved matches `"'_"`, store
`DefAnonRegion` as the resolution result.
Small caveat: this check has to be done only if the lifetime has not been
already resolved, because `'_` is a legal lifetime in stable Rust, e.g.:
```rust
fn foo<'_, T>(xs: &'_ [T]) -> &'_ T { &xs[0] }
```

In `typeck::astconv`: if a lifetime has been resolved to `DefAnonRegion`,
return the same region that would be used for `'a` if omitted in `&'a T`.

Example:
```rust
struct Context<'a, 'left: 'a, 'right: 'a> {
    left: &'a Inner<'left>,
    right: &'a Inner<'right>,
}
fn left<'a, 'left>(cx: Context<'a, 'left, '_>) -> &'a Inner<'left> {
    cx.left
}
fn right<'a, 'right>(cx: Context<'a, '_, 'right>) -> &'a Inner<'right> {
    cx.left
}
```

# Drawbacks

It might be confusing that `'_` *only* has special semantics attached to it
if it's not declared. It's possible `'_` shouldn't have ever made it into
stable Rust, and it could be removed as no uses are expected in the wild.
It could also be linted against - or the new semantics could be opt-in even
in stable Rust, if we add a mechanism for that.

# Alternatives

Do nothing and suffer the pain of a few hundred unnecessary explicit lifetimes.

# Unresolved questions

What to do with the existing `'_`?
Can a feature-gated implementation be merged if it's not observable in stable?
