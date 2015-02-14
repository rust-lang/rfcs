- Feature Name: `item_grouping`
- Start Date: 2015-02-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow an anonymous `impl { ... }` for grouping items without introducing a module.

# Motivation

Many libraries today contain code like

```rust
#[cfg(target_os = "windows")]
mod bar;

#[cfg(target_os = "windows")]
pub use bar::Baz;

#[cfg(target_os = "windows")]
fn quux() { ... }
```

With the new syntax, we can eliminate the duplication of `cfg` directives:

```rust
#[cfg(target_os = "windows")]
impl {
    mod bar;

    pub use bar::Baz;

    fn quux() { ... }
}
```

This makes it easier to see at a glance which items are enabled when.  It's not
a module, so you don't have to deal with another abstraction boundary or add a
bunch more `use` items.

Note that [`#[cfg]` on inherent `impl`s](http://is.gd/Jp2qVB) already works.

Syntax extensions can similarly use this to process a set of items as a unit.
For example I could rework [dynamodule][] to accept

```rust
#[class] impl {
    struct Bicycle(&'static str);
    type is_a = Vehicle;

    fn new(color: &'static str) {
        Bicycle(color)
    }

    fn go_somewhere(&self) -> String {
        format!("{} bicycle has a basket and a bell that rings", self.color)
    }
}
```

Instead of digesting a whole function as token trees, we let libsyntax parse
the `impl { ... }` normally, then fix up the AST at a higher level.
(Naturally, this is only doable with procedural macros at the moment.)

[dynamodule]: https://github.com/kmcallister/dynamodule#overview

# Detailed design

`impl { ... }` just provides a unit of grouping for `cfg`, syntax extensions,
and similar purposes.  The items inside `impl { ... }` behave in every respect
as though they were defined in the enclosing scope.

(Of course, syntax extensions can perform arbitrary modifications to the AST.
The `type is_a` above would *not* be defined in the enclosing scope, because
the syntax extension interprets it specially.)

The decision to support attributes besides `#[cfg]` can be made in the future
on a case-by-cases basis.  With this PR alone,

```rust
#[derive(Clone)]
impl {
    struct Foo;
    struct Bar;
}
```

is no more legal than today's

```rust
#[derive(Clone)]
impl S { }
```

After a quick look through the [attributes list][], my conclusion is that most
attributes, if they make sense at all in this context, would be "distributive".
That is, they would be shorthand for applying the same attribute to each item
within.

In general, I don't think we should allow this shorthand, as it changes the
meaning of items in a rather implicit / non-local way.  For the lint attributes
in particular (`allow`, `warn`, `deny`, and `forbid`), the distributive
interpretation seems useful and un-problematic.  This RFC adopts such an
interpretation of just these four attributes, alongside the usual meaning of
`cfg`.

The only other built-in attribute I think we may want to support is

```rust
/// doc comment
impl {
    ...
}
```

to designate "sections" in rustdoc output.  But I don't find this very
compelling at the moment, and I don't include it as part of the proposed
change.

[attributes list]: http://doc.rust-lang.org/reference.html#attributes

# Drawbacks

Complexity blah blah.  The syntax `impl { ... }` is so non-specific that it's
hard to imagine using it for anything else.

# Alternatives

Item grouping, at least for the purposes of `cfg` stripping, could be a syntax
extension.  The code using it would be (in my opinion) less clear, and it would
still need to be a compiler built-in or else not available on the stable
releases.

The no-keyword alternative

```rust
#[cfg(target_os = "windows")] {
    mod bar;

    pub use bar::Baz;

    fn quux() { ... }
}
```

conflicts with allowing attributes on blocks.  Within a block, both blocks and
(groups of) items may appear.

# Unresolved questions

None.
