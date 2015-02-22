- Feature Name: explicit_fun_ptr
- Start Date: 2015-2-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

It is likely that Rust will want to someday have a notion of an unboxed function type to be used
with custom pointer types.<sup>1</sup> Ideally then today's function pointers would become `&'static`s of
functions. But this is a breaking syntactic change, and unboxed functions can probably not be exposed
by 1.0. This RFC proposals making the needed breaking syntactic change now, so unboxed functions can
be added---and function pointers turned to borrowed pointers---all backwards compatibly in the
future.

# Motivation

Currently, functions have statics lifetimes, and function pointers act like `&'static`s. This fits
common case where all functions are either statically linked or dynamic linked in a transparent
fashion.

When more manually invoking the loader or JIT-compiling, however, functions with non-static
lifetimes arise as function are dynamically creatable and destroyable like any value of a "normal"
type. As such, it would be nice to have all the machinery Rust offers (lifetimes, ownership, etc)
available for functions too.

A nice way to enable all that machinery is add actual "unboxed" function types, unlike the
function-ptr types that currently exist. That ways the whole menagerie of parameterized pointer
types are available for use with functions to generate many types of function pointers. To give some
examples of the possibilities, `Arc` can be used ARC to unload libraries or free JITed code, or, as
@eddyb came up with, (unsafe to deref) `*const` pointers to functions could replace unsafe function
pointers.

Adding unboxed function types begs the questions of what would happen to the current function
pointers were that route taken. Since they act just like immutable borrowed static pointers to
functions, it would be nice if they became that in actuality, lest two ways to do the same thing be
introduced so deeply in the language. Unfortunately, combining the two requires a breaking syntactic
change. There are many details that need to be resolved to support unboxed function types, plus
probably a good deal of implementation work, so it is probably not realistic to try to do this
before 1.0. Note, while it is tempting to use DST for this, but unfortunately that would make
function pointers fat, and moreover the sizes of functions are not in-general known dynamically
anyway (c.f. foreign functions).

So if this requires a breaking change to make nice, yet is too big to do before 1.0, what can be
done? My suggestion is simply to simply change the syntax of function pointer types to make clear
they act like `&'statics`. This doesn't force any semantic changes in the future, but opens to door
to reusing `fn(...)...` for unboxed functions by allowing function pointers to silently become
actual `&statics` if/when function types are added in the future. Meanwhile Rust, while slightly
less ergonomic---function pointers are quite rare, is more forthright about its current
expressiveness today.

# Detailed design

Quite simply, change the syntax of function pointer types from

```rust
('extern' STRING_LIT?)? 'fn' '(' (IDEN ':' TYPE (',' IDEN ':' TYPE)* ','?)? ')' ('->' TYPE)?
```

to

```rust
'&' '\'static' ('extern' STRING_LIT?)? 'fn' '(' (IDEN ':' TYPE (',' IDEN ':' TYPE)* ','?)? ')' ('->' TYPE)?
```

Like slices before DST, the `fn(...) ...` itself will not denote a type on its own and need not even
parse.

In the future, `fn(...) ...` can be made legal syntax and given semantics, in which case the current
syntax would no longer be its own grammatical production but remain valid, and actually denote the
type of a static, immutable borrowed pointer to a function.

Just as today, function items may have unique function types, but those types won't have a surface
syntax, and expressions (in practice just identifiers/paths) of those types will coerce to function
pointers (see [1]).

# Drawbacks

 - Temporary ergonomic loss
 - May not extend language with borrowed function pointers in the future
 - All non-static functions can only be created in unsafe ways, so this is just making a safe
   interface to fake at the end of the day.
   - But even if the function itself may not conform to the type, at least the backing executable
     memory could potentially be managed 100% safely.

# Alternatives

 - Seeing that in the short-term, all functions will have a static lifetime (i. e. can only
   transmute a value to a function pointer and not a function). It might be perfectly safe to allow
   `&fn(...)  ...` with the normal lifetime inference rules. I hope this is the case, but as it
   relies on more assumptions I put it as an alternative to stay safe.

 - The above isn't true, but the lifetime inference could special-case-default to `static` with a
   function pointer type. (Yuck!)

 - `fn (...):'a ...` syntax. This would seem to doom us to a redundancy with borrowed pointers in
   the future rather than prevent it, thought it does give us more expressiveness. Pre-RFC for this
   at http://internals.rust-lang.org/t/pre-rfc-fn-lifetimes/472 .

# Unresolved questions

Perhaps I am mistaken and the semantics of `&'static`s and functions pointers differ.

[1]: https://github.com/rust-lang/rust/pull/19891 The implementation currently distinguishes between
     function types and function pointer types, but this is not really exposed as part of the language.
