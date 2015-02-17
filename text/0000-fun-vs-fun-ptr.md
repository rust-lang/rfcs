- Feature Name: explicit_fun_ptr
- Start Date: 2015-2-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

For clarity and safety, it is likely that Rust will want to someday distinguish between functions
and function pointers[1]. It is tempting to use DST for this, but unfortunately that would make
function pointers fat, and moreover the sizes of functions are not in-general known dynamically
anyway (c.f. foreign functions). Thus, this RFC instead proposals a slight syntactic disambiguation
that should be much more forwards compatible with future work in this area post-1.0.

# Motivation

Currently, functions are treated like statics, and function pointers like `&'static`s. This fits
common case where all functions are either statically linked or dynamic linked in a transparent
fashion.

When more manually invoking the loader or JIT-compiling, however, these restrictions no longer fit
as function are dynamically creatable and destroyable like any value of a "normal" type. As such, it
would be nice to have all the machinery Rust offers (lifetimes, ownership, etc) for keeping track of
values of normal types available for values of function pointer types.

Implementing all these things for functions will mean much more decision making and implementation
effort than is feasable for the pre-1.0 timeframe. What we can do now is change the syntax of
function pointer types to make clear they really are `&'statics`. This will syntactically "make way"
for the other function pointer types to be added in the future, and make Rust more forthright about
its current expressiveness.

# Detailed design

Quite simply change the syntax of function pointer types from

```rust

fn(...) ...
```

to

```rust

&'static fn(...) ...
```

Like slices before DST, the `fn(...) ...` itself will not denote a type on its own and need not even
parse. It can be made legal syntax and given semantics in the future.

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

 - Seeing that in the short-term, all functions will have a static lifetime (i. e. can only cast a
   value to a function pointer and not a function). It might be perfectly safe to allow `&fn(...)
   ...` with the normal lifetime inference rules.

 - The above isn't true, but the lifetime inference could special-case-default to `static` with a
   function pointer type. (Yuck!)

 - `fn (...):'a ...` syntax.

# Unresolved questions

Perhaps I am mistaken and the semantics of `&'statics` and functions pointers differ.

[1]: https://github.com/rust-lang/rust/pull/19891 The implementation currently distinguishes between
     function types and function pointer types, but this is not really exposed as part of the language.
