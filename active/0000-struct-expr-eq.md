- Start Date: 2014-07-17
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC proposes to replace the `:` token in struct expressions with `=`. For example,
```rust
let p = Point { x: 3, y: 5};
```
will be replaced with
```rust
let p = Point { x=3, y=5 };
```

At the first stage, using `:` instead of `=` will issue a warning. A tool will
be provided that gets the output of the build process, finds those warnings,
and automatically updates all struct expressions.

A proof-of-concept implementation is available
[here](https://github.com/noamraph/rust/tree/struct-expr-eq).

# Motivation

This is obviously a minor issue, but to me it seems like a quite visible wart
that could still be
fixed without too much pain. A `=` between the field name and the value in struct
expressions just seems a much better fit than `:`, given the context of the Rust
language and other languages with similar syntax. Before
[RFC 25](https://github.com/rust-lang/rfcs/blob/master/complete/0025-struct-grammar.md),
using `=` instead of `:` wouldn't have worked, because `=` would have created
an ambiguity between a struct expression and a block with an assignment, but now
the token can be chosen to be whatever seems fit.

In other languages with a similar construct, `=` is more widely used.

In OCaml:
```ocaml
type point = {x: int; y: int};;
{ x=3; b=5 };;
```

In Haskell:
```haskell
data Point = Pt {x, y :: Float}
Pt {x=3, y=5}
```

In Python:
```python
Point = namedtuple('Point', 'x y')
Point(x=3, y=5)
```

In Rust itself, beside struct expressions, the pattern `A: B` is used for declaring
types and for declaring type boundaries. In both of these cases, `A: B` can be
read as "`A` is a `B`", or "`A` is a member of the set `B`". That is, the more
general entity is on the right. This certainly can't be said of `x: 3`.

On the other hand, `=` is used mainly for assignments. In
`let p = Point { x=3, y=5 }` it can certainly be said that `p.x` is assigned
the value `3`. Another usage of `=` is in keyword arguments (Currently in the
`fmt!` family of macros, perhaps some day in general functions). The use of `=`
for keyword arguments also aligns well with using `=` for
struct expressions, since the form `Point { x=3, y=5 }` can be seen as a special
constructor which accepts a keyword argument for each field.



# Detailed design

The `:` token in the struct expressions syntax will be replaced with a `=`, so the
new definition will be:

```
struct_expr : expr_path '{' ident '=' expr
                      [ ',' ident '=' expr ] *
                      [ ".." expr ] '}' |
              expr_path '(' expr
                      [ ',' expr ] * ')' |
              expr_path ;
```

At the first stage, using `:` will be accepted as it currently is, and will
issue a warning:

> warning: Use '=' instead of ':' in struct expressions. You can use the
> rust-update-structs tool to automate the replacement process.

The pretty printer will be updated to output `fieldname=value` instead of
`fieldname: value`.

I prefer `x=3` over `x = 3` because:
 * This seems to be the common pattern in OCaml, Haskell and Python.
 * It helps distinguish between an actual assignment to a variable and a
   struct expression, which is, well, an expression.
 * It is shorter.

A tool, `rust-update-structs` will be provided and installed alongside with
`rustc`. It will be named `rust-update-structs` to ease finding it with tab-completion.
It's usage will be:

    rust-update-structs [--dry-run] path-prefix build-output-filename

It will search, using a regex, the file `build-output-filename` for the above
warnings, and collect filenames and line and column numbers. It will then go
over all the files that a prefixed by `path-prefix` and verify that a `:` is
indeed where it is expected. Then, unless `--dry-run` is given, it will go
over all those files and do the actual replacement. If a replaced colon is
followed by a space, the space would be removed, in order to convert the common
spacing `x: 3` into `x=3`.

# Proof-of-concept implementation

A proof-of-concept implementation is available at
https://github.com/noamraph/rust/tree/struct-expr-eq. About 7600 lines were updated,
the vast majority of those automatically using the `rust-update-structs` tool.
A few struct expressions which appear in macro definitions had to be updated
manually.

Known issues with the proof-of-concept:
 * The language reference, and perhaps other docs, need to be updated (code
   segments that get compiled by rustdoc were updated)
 * The `rust-update-structs` tool should be added to the build and installation
   process.

In order to compile the final revision, a snapshot must be first created from
the
[support-struct-expr-eq](https://github.com/noamraph/rust/tree/support-struct-expr-eq)
tag.

# Drawbacks

As with any wide-ranging change, this will require some effort for upgrading.
For most code the replacement could be entirely automatic. Pull requests created
before the change will have to be updated (using the `rust-update-structs` tool)
in order to merge cleanly.

There are no known drawbacks of the proposed syntax itself, when compared to the
current syntax.

# Alternatives

We could discard this RFC, and live happily ever after with the colons in struct
expressions.

# Unresolved questions

Should the name be "struct expressions" or "struct literals"?

