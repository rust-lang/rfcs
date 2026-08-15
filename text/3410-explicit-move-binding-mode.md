- Feature Name: explicit_move_binding_mode
- Start Date: 2023-04-07
- RFC PR: [rust-lang/rfcs#3410](https://github.com/rust-lang/rfcs/pull/3410)

# Summary
[summary]: #summary

Enable the use of the `move` keyword to explicitly specify the moving binding
mode in patterns. This allows users to opt out of match ergonomics for some but not all bindings.

Warn about unnecessary keywords that specify binding mode (called “specifiers” in this document).

# Motivation
[motivation]: #motivation

Currently, there are multiple binding modes in patterns, but only some are explicitly specifiable.
This is an obvious inconsistency, as match ergonomics permit changing the
default binding mode of a pattern. Changing it back is only natural, as changing it
to the non-default mutable move is possible—that is, writing `mut` overrides match ergonomics
and performs a move after the dereference, although the resulting binding is mutable.

Specifically, when most bindings of a large pattern should be of one binding mode,
but some should be moves, it is inconvenient to forgo match ergonomics entirely
and repeatedly use `ref` or `ref mut` specifiers.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Expert explanation

The `move` keyword resets the binding mode for an individual identifier pattern
to the moving mode. The meaning of `mut` remains the same.
The matching still dereferences by match ergonomics rules.

## Beginner explanation

When deconstructing a value, it is sometimes desirable to get a reference to the
element bound to a variable instead of moving that value into the variable. To do
this, you can set the _binding mode_ of an identifier pattern by prefixing it with
`ref`, `ref mut` (or `move`, but this is the default). You can also use this syntax
to make a plain move mutable, by prefixing with just `mut`.

```rust
let mut possibly_x: Option<i32> = Some(37);

if let Some(ref mut x) = possibly_x {
    *x += 2;
} // Here we use `ref mut` to get a mutable
  // reference to the value contained in `possibly_x`

match possibly_x {
    None => {
        println!("No value found!");
        println!("Can’t work with non-existant value.")
    }
    Some(mut x) => {
        println!("The value is {x}.");
        x += 2;
        println!("That value plus two is {x}.");
    } // Here we use `mut` to mark the binding
      // as mutable, allowing us to modify it
      // Note that this does not change the value
      // inside `possibly_x`, as we did not use `ref mut`
}
```

_Match ergonomics_ allow you to more easily get references to bindings in patterns.
When a pattern that is not a reference pattern or a pattern that can match anything (`_` or identifier patterns)
is matched against a value that is a reference, the value is automatically dereferenced,
as though the pattern had been written `&<pattern>` or `&mut <pattern>`, and the default
binding mode is set to `ref` or `ref mut`.
All identifier patterns (`x` and the like) that don’t have an explicit binding mode
instead bind with binding mode `ref` or `ref mut`.
This means that `let (x, y) = &a` is the same as `let &(ref x, ref y) = &a`.  
We can rewrite the above example as follows:

```rust
let mut possibly_x: Option<i32> = Some(37);

if let Some(x) = &mut possibly_x {
    *x += 2
} // Here, `x` has the type `&mut i32`
```

You can opt out of this behaviour for individual identifier patterns by prefixing
them with `move` or `mut`. Note that you cannot create a mutable reference from an
immutable one, and this is not what `mut` does—it sets the binding mode to a mutable move.

```rust
let mut x_and_y: (i32, i32) = (25, -4);

let (x, mut y) = &mut x_and_y;
// The type of `x` is `&mut i32` and
// the type of `y` is `i32` (and the binding is mutable)

*x += 2; // `x_and_y` is modified
y += 2; // `x_and_y` is not modified

let (move x, y) = &x_and_y;
// The type of `x` is `i32` and
// the type of `y` is `&i32`
```

You can also switch to an immutable reference binding mode by using the `ref` keyword.

```rust
let mut x_y_z: (i32, i32, i32) = (400, 1, -99);
let (x, ref y, move z) = &mut x_y_z;
// The type of `x` is `&mut i32`,
// the type of `y` is `&i32` and
// the type of `z` is `i32`
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The syntax for _IdentifierPattern_ is updated as follows:
> _IdentifierPattern_:
> 
> (`ref` | `move`)? `mut`? IDENTIFIER (`@` _PatternNoTopAlt_)?

The binding mode of a binding depends on the default mode and
the binding mode specifier (`mut`, `move`, `ref`, or `ref mut`)
and is described by the following table. If the entry into the table is 
followed by an exclamation mark in parentheses (“(!)”), a lint is triggered.
The symbol “"” indicates that the entry is the same as the entry to the left,
excluding whether it triggers the lint.

| ↓specifier | →default = move       | reference | mutable reference |
|------------|-----------------------|-----------|-------------------|
| `mut`      | move mutable          | "         | "                 |
| `ref mut`  | mutable reference     | "         | "  (!)            |
| `ref`      | reference             | "  (!)    | "                 |
| `move`     | move (!)              | "         | "                 |
| _none_     | move                  | reference | mutable reference |

The lint is controlled by `unnecessary_binding_mode`. It is warn-by-default.

# Drawbacks
[drawbacks]: #drawbacks

- It can be argued that use of the `move` keyword should be replaced with
  use of the `ref` keyword and not using match ergonomics at all.
- This further entrenches the unintuitive fact that `mut` sets the binding
  mode to a mutable move and disables referencing.
- This means that either the grammar gets more complicated or an additional
  sequence is allowed by the grammar but disallowed by the compiler.
- The `move` keyword can be confusing here because a copy may happen instead.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I believe the `move` keyword is an excellent candidate for syntax here,
as it already exists and is also used by the match ergonomics RFC.  
Alternative keywords would be `const` or `let`.
Alternatively, a new keyword could be added, although this would need to be
a soft keyword or happen over an edition boundary.
Possible keywords are `copy`, `bind`, `value`, `free`, `new`, `just`, `get`, `fresh`, `set`, `var`.

---

An alternative to this proposal is to update match ergonomics such that a non-reference
pattern matched against a reference does not update the binding mode, but instead
matches the subpatterns against borrowed subvalues. This would allow writing this:

```rust
let x_and_y: (i32, i32) = (-9, 2);
let (x, &y) = &x_and_y;
// `x` is of type `&i32`, while
// `y` is of type `i32`
```

Then, the `mut` keyword could be used to make the binding itself
mutable instead of the reference the binding binds.

```rust
let mut a = 3;
let mut x_and_y: (i32, i32) = (77, 0);
let (mut x, y) = &mut x_and_y;
*x += 2; // `x_and_y` is modified
x = &mut a;
*x += 2; // `a` is modified
```

Note that this proposal likely requires an edition boundary unless
complicated backwards-compatibility measures are employed.

---

A similar possibility for the current proposal: The combination `mut ref` could
be added distinctly from `ref mut` to make the binding mutable, not the reference.

# Prior art
[prior-art]: #prior-art

None that I know of. Other languages don’t have match ergonomics as far as I know.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How should the combination `move mut` be handled? Should it generate an error or be warned against, working as if it was bare `mut`?
I believe having the combination of `move` vs `ref` and `mut` vs nothing be as simple as concatenation
could be useful for macros.

It is also possible to change the preferred way of indicating a mutable move to `move mut`
and warn against plain `mut` instead, as plain `mut` switching to moves is unintuitive.

---

What should the warnings/errors look like?
Here are some ideas:

### Error for `move mut`:

```none
error: move semantics can't be specified here, use bare `mut` instead
 --> src/main.rs:4:10
  |
4 |         (move mut x, y, z) => {
  |          ----
  |          |
  |          help: remove this `move`
  |
```

### Unnecessary `move`:

```none
error: unnecessary binding mode specifier
 --> src/main.rs:4:10
  |
4 |         (move x, y, z) => {
  |          ^^^^
  |          |
  |          `move` not required here because it's implied
  |
  = note: `#[warn(unnecessary_binding_mode)]` on by default
```

(inspired by E0449)

### Unnecessary `ref`:

```none
  warning: ref semantics don't need to be specified here because you're matching against a reference
 --> src/main.rs:4:10
  |
3 |     match &(a, b, c) {
  |           ^---------
  |           |
  |           reference originates here
  |
4 |         (ref x, y, z) => {
  |          ---
  |          |
  |          help: remove this `ref`
  |
  = note: `#[warn(unnecessary_binding_mode)]` on by default
```

Note that the author is not intimately familiar with the style of `rustc` error messages, so these likely need adjustment.

# Future possibilities
[future-possibilities]: #future-possibilities

It is somewhat unintuitive that the `mut` specifier sets the binding mode to a mutable move.
It would be possible to update match ergonomics in a future edition of Rust to have specifiers
_modify_ the binding mode instead of setting them, or require `move mut` instead of
just `mut` for mutable moves (see also the alternatives section).

It would also be possible to demote `mut` from the status of binding mode specifier and instead
have it be completely orthogonal to binding mode.
