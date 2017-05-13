- Start Date: 2014-06-09
- RFC PR #:
- Rust Issue #:

# Summary

Resolve paths of the form `::foo::...` when compiling the crate named `foo`.

# Motivation

`libstd` has

~~~ .rs
// A curious inner-module that's not exported that contains the binding
// 'std' so that macro-expanded references to std::error and such
// can be resolved within libstd.
#[doc(hidden)]
mod std {
    // mods used for deriving
    pub use clone;
    pub use cmp;
    pub use hash;

    ...
~~~

so that e.g. references to `::std::cmp::Equal` produced by macros will work both inside `libstd` and elsewhere.

This is a hack, and it requires an explicit list of all the modules to be used from macros.

I'm adding something similar to `librustc` for lint-related macros.

# Detailed design

* When compiling the crate named `foo`,
* if there is no item or crate named `::foo`, and
* there is an item (not another crate) named `::bar`,

then allow the absolute path `::foo::bar` as an alias for `::bar`.

Relative paths such as `foo::bar` and `super::bar` are not affected.  Macro expansions should always use absolute paths, but they are uncommon in other code, which limits the unintended effect of this change.

# Drawbacks

Complicates name resolution rules.  Possibility for confusion when a crate and a top-level module have the same name.

# Alternatives

Explicit syntax to mean "drop this path component if it matches the current crate name", e.g. `::std?::cmp::Equal`.

Proper hygeinic capture of crates (sounds hard).

# Unresolved questions

Should we allow relative paths like `foo::bar` as well?  I don't know of a use case, but it's more consistent in a sense.
