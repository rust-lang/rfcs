- Start Date: 2014-11-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Preemptively reserve a class of `$` identifiers for allowing backwards compatible improvements to the macro system.

# Motivation

While at the moment within macros identifiers starting with `$` are used for arguments, 
we may in the future need some builtin "special" identifiers that can be used within macro definitions. Additionally,
it may be useful for a higher order macro design in the future.

# Detailed design

Disallow declaring macro arguments using identifiers starting with `$_` and/or `$$` (The second symbol can be bikeshedded). `$$` is already disallowed, though it would help to have a clearer error message on that. In general, disallow such identifiers anywhere in a place that accepts arbitrary token trees.

For example, [this rfc](https://github.com/rust-lang/rfcs/pull/453) would benefit from a `$_crate` "magic word" to  mean the local crate. 
In that specific case, since `crate` is a keyword, [`$crate` is fine](https://github.com/rust-lang/rfcs/pull/453#issuecomment-62291026), however in general this may not work for all extensions to the macro system. [This design](http://discuss.rust-lang.org/t/pre-rfc-auto-derive-for-generating-deriving-decorators/709) might require some "magic" identifiers like `$_name` which mean something special to the compiler, for example, and this RfC would prevent it from making a non-backcompat change post 1.0.

A future proposal to fix a problematic `concat_idents!` could be to introduce a `$_concat()` magic method. (In general, there are many such magic methods that would be useful for higher order macros as well).



# Drawbacks

We lose a small subset of the possible identifiers that can be used for declaring macro arguments.

# Alternatives

The alternative is to continue with the current system, and perhaps use `$$foo` or some other symbol (`$@foo`/`$!foo`?) if we ever need this.

# Unresolved questions

The symbol needs bikeshedding. I prefer `$_foo`, though `$$foo` is already disallowed. Perhaps reserving both would be the best -- one can be used for higher order macros, the other for magic builtins.
