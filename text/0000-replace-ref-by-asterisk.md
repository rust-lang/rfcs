- Start Date: 2015-01-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Change `ref` in patterns to `*`, to make pattern syntax better match expression syntax.
Example: `Some(ref x)` becomes `Some(*x)`.
This change removes all the usage of `ref` keyword,
so the RFC also discusses the fate of it.

# Motivation

In current design, pattern syntax tries to mirror expression syntax
(with some unavoidable `mut` related exceptions), so one may think that
"everything works backwards". But `ref` is an exception here, because:

* It doesn't appear in expression syntax at all.
* It works "forwards" in contrary to rest of pattern syntax.

Such an exception may be confusing for a newcommer and even
some more experienced Rust programmer may feel like `ref` doesn't fit pattern syntax well.
Additionaly, one can read both `ref` and `&` as *ref*, but they serve totally different purposes.
That is another source of confusion.

This RFC proposes to change `ref` to `*` to make this exception disappear.

Following example shows the symmetry of patterns and expressions in proposed design.
Notice that
the syntax used to reconstruct `x` value is exactly the same as the syntax used to match on in.

```rust
let x = Some(5);
if let Some(*y) = x {
    assert_eq!(Some(*y), x)
}
```

Another motivation are opinions, heard from time to time, that `ref` should be called `deref`.

Yet another reason for this change is discoverability -- it's reusing a operator
(while mostly keeping its semantics),
so it's not unlikely for somebody to discover this syntax without consulting a manual.
Actually, there are some users reporting that they tried to use `*` before learning about `ref`.

# Detailed design

### Interaction with `mut`

This RFC proposes to change (in contrary to what was suggested before, `mut ref x` is currently not a valid pattern):

```rust
ref x            into       *x
ref mut x        into       mut *x
```

Notice that in addition to simply changing one token to another,
this RFC also proposes to change placing of `mut` keyword.
The reason for that change is to make `mut` refer to the
thing on the right of `mut`. `mut` keyword in `mut *x` means "dereference of `x` is mutable",
because it parses as `mut (*x)`.

Syntax may allow the parenthesised version too, for consistency / increase of readability.

Another problem is interaction with `&mut _` pattern. In the new design,
following would be ambigous to parse:

```rust
&mut *x
```

It could mean either `&mut (*x)` of `&(mut *x)`. Fortunately, under no conditions
right one could compile, so the left version could be assumed.
If the user wanted to have a mutable reference in this case,
they have to write

```rust
&mut mut *x
```

which is unambigous and corresponds to today's `&mut ref mut x`. The double `mut` may seem weird,
but fortunately this pattern is not a common one – it's used only 6 times in rust source,
from which only one is outside tests.
Additionaly this pattern (at least according to the author's knowledge) could be replaced by just `x`.

### Fate of the `ref` keyword

The `ref` keyword could simply lose its keyword status,
making it usable as eg. variable or function name
(which might be desired), or it could be repurposed.

The keyword may be used in some context which deals with
abstracting over different types of references (eg. mutable vs shared or Box vs Rc).

Other way to repurpose the `ref` keyword is to use it in patterns
as generic deref. Examples:

```rust
let ref x = Box::new(5); // x == 5
if let ref [1, _] = vec![1, 2] {}
let rc = Rc::new("abc".into_string());
let ref *s = rc; // s has a type &String
```

Note that this suggestion is not a part of change proposed by the RFC.

# Drawbacks

* This is a breaking change, which touches a lot of Rust programs. As Rust is currently already
  in alpha, this may be considered a major drawback. However, the change is rather simple
  and can be introduced rather painlessly:
    * Code upgrade can be done with a simple find and replace (only valid usages of `ref` are in patterns).
    * Before beta, old syntax could be accepted while emitting a warning with explanation of change.
      It could be also done by making both old and new syntaxes valid and providing a lint for using
      the old one. The lint could default to warn for alpha and forbid for beta.
  The change has to be reflected also in documentation.
* Ungoogleability. Words *star* and *asterisk* are quite googleable too, although they are more common than *ref*.

# Alternatives

* Do nothing.
* Keep old `mut` order. That makes the change simpler, but (at least for ther author) more confusing.
  Also, that would make `*mut` appear more often in patterns, which may be confusing, since
  it looks as raw pointer type.

# Unresolved questions

* Should we allow additional parenthesis (such as `mut (*x)`) in the syntax?
* Should we introduce `*(mut x)` pattern which would put the reference in a mutable slot?
* Fate of the `ref` keyword (should it be repurposed, reserved or just removed?).
