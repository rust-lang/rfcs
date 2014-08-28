- Start Date: 2014-08-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Macros which resolve to idents should be usable in all positions that expect idents, including function/struct/enum names in definitions, et cetera.
Currently they can only be used as variable names.

# Motivation

This will allow for creating better macros which resolve to functions. For example:

```
macro_rules! make_setter(
    ( $attr:ident ) => (
        fn concat_idents!(set_,$attr) (&self) -> u32 {
            self.$attr
        }
    );
)
```

can be created to generate setters just by calling `make_setter(foo)` (which will create `set_foo()`)


Currently, using macros in this manner will lead to errors like

```
test.rs:1:17: 1:18 error: expected `(` but found `!`
test.rs:1 fn concat_idents!(Foo, Bar) () {
                          ^
```

See also: https://github.com/rust-lang/rust/issues/13294

# Detailed design

This will probably require converting [`ast::Ident`](http://doc.rust-lang.org/master/syntax/ast/struct.Ident.html) to an enum allowing for a `NamedIdent` vs `MacroIdent` duality.

# Drawbacks

None that I see

# Alternatives

Suggestions welcome

# Unresolved questions

I'm rather unsure about the implementation.
