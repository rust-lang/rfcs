- Feature Name: `macro_escape_char`
- Start Date: 2015-03-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow `$` to escape `+` and `*` so that they can be used as separators.

# Motivation

Make it possible to write trait bounds more easily, e.g.

```rust
macro_rules! foo {
    ($($t:ident),+) => {
        fn bar<T: $($t)++>(t: T) -> T { ... }
        //             ^ currently invalid to separate with `+`.
    }
}
foo!{Copy, Send, Sync}
```

# Detailed design

Use `$` to escape `*`, `+`, `?` and `$` when used as separators in the macro matcher and transcriber. For example,

```rust
macro_rules! t {
    ($($t:ident)$**) => {
        x!($(1 $t)**);
        y!($(2 $t)$**);
        z!($(3 $t)$+*);
        w!($(4 $t)$$*);
    }
}

// t! will match this:
t!(foo * bar * baz);

// expand to:
x!(1 foo 1 bar 1 baz *);   // note that without escaping it will treat as no separators
y!(2 foo * 2 bar * 2 baz); // escaping the * to make it a separator instead of a Kleene star
z!(3 foo + 3 bar + 3 baz); // same for +
w!(4 foo $ 4 bar $ 4 baz); // use $ to escape another $.
```

Outside of separators, `$+`, `$*`, `$?` and `$$` will be treated as the same as `+`, `*`, `?` and `$` but with no special meanings:

```rust
macro_rules! u {
    () => { $+ $* $$ $$crate }
}

u!{}

// expand to:

+ * $ $crate
```

A `$` followed by neither `+`, `*`, `$`, `?`, the keyword `crate`, identifiers or brackets is an error, allowing room for future expansion.

# Drawbacks

The macro grammar becomes a bit more complicated.

# Alternatives

* Do nothing, as expansion using `+` and `*` as separators may be simulated using a dedicated macro, and while matching using `+` and `*` as separators are impossible, we could change the macro interface to this restriction.
* The escape character `$` may be changed to other symbols like `\\`.
* No special treatment of `$+`, `$*`, etc. when not used as separators.

# Unresolved questions

None

