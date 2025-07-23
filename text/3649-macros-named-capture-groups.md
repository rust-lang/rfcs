- Feature Name: `macros-named-capture-groups`
- Start Date: 2024-05-28
- RFC PR: [rust-lang/rfcs#3649](https://github.com/rust-lang/rfcs/pull/3649)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

It will now be possible to give names to capture (repetition) groups in macro
patterns, which can then be referred to directly in the macro body and macro
metavariable expressions.

_Rustc usually refers to these groups as "repetitions" in diagnostics. This
RFC uses "capture groups" which is more general (they don't always repeat),
and more in line with regex._

# Motivation
[motivation]: #motivation

Rust has no way to refer to capture groups directly, so it uses the variables
they capture to refer to them indirectly. This leads to confusing or limited
behavior in a few places:

- Expansion with multiple capture groups is extremely limited. In many cases,
  the ordering and nesting of different groups is restricted based on what can
  be inferred by the contained variables, since the groups themselves are
  ambiguous.
- Repetition-related diagnostics are suboptimal because the compiler has limited
  ability to guess what a capture group _should_ refer to when a captured
  groups and variables do not align correctly.
- Repetition mismatch diagnostics can only be emitted after the macro is
  instantiated, rather than when the macro is written. (E.g. "meta-variable
  `foo` repeats 2 times, but `bar` repeats 1 time")
- As a result of the above, using repetition is somewhat fragile; small
  adjustments can break working patterns with little indication of what exactly
  is wrong. Reading code with multiple capture groups can also be confusing.
- Metavariable expressions as they currently exist use an unintuitive format:
  syntax like `${count($var, n)}` is used to refer to the `n`th ancestor group
  of the smallest group that captures `$var`. Referring to groups directly would
  be more straightforward than using a proxy.

It is expected that named capture groups will provide a way to remove ambiguity
in expansion and metavariable expressions, as well as unblock diagnostics that
do a better job of guiding the macro mental model.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Capture groups can now take a name by providing an identifier between the `$`
and opening `(`. This group can then be referred to by name in the expansion:

```rust
macro_rules! foo {
    ( $group1( $a:ident ),+ ) => {
        $group1( println!("{}", $a); )+
    }
}
```

This would be approximately equal to the following procedural code:

```rust
let mut ret = TokenStream::new();

// Append an expansion for each time group1 is matched
for Group1Captures { a } in group1 {
    ret += quote!{ println!("{}", #a); };
}
```

Named groups can be used to create code that depends on nested repetitions:

```rust
macro_rules! make_functions {
    (
        // Create a function for each name
        names: [ $names($name:ident),+ ],
        // Optionally specify a greeting
        $greetings( greeting: $greeting:literal, )?
    ) => {
        $names(
            // Create a function with the specified name
            fn $name() {
                println!("function {} called", stringify!($name));
                // If a greeting is provided, print it in every function
                $greetings( println!("{}", $greeting) )?
            }
        )+
    }
}

fn main() {
    make_functions! {
        names: [foo, bar],
        greeting: "hello!",
    }

    foo();
    bar();

    // output:
    // function foo called
    // hello!
    // function bar called
    // hello!
}

```

This expansion is not easily possible without named capture groups because
of ambiguity regarding which groups are referred to.

Expansion of the above will approximately follow this procedural model:

```rust
let mut ret = TokenStream::new();

// Append an expansion for each time group1 is matched
for NamesCaptures { name } in greetings {
    let mut fn_body = quote! { println!("function {} called", stringify!($name)); };

    // Append the greeting for each
    for GreetingCaptures { greeting } in greetings {
        fn_body += quote! { println!("{}", #greeting) };
    }

    // Construct the function item and append to returned tokens
    ret += quote! { fn #name() { #fn_body  } };
}
```

Groups can also be used in the expansion without `(...)` to emit their entire
capture.

This works well with a new "match exactly once" grouping that takes no kleene
operator (as opposed to matching zero or more times (`*`), matching once or
more (`+`), or matching zero or one times (`?`)).

```rust
macro_rules! rename_fn {
    (
        $newname:ident;
        $pub(pub)? fn $oldname:ident( $args( $mut(mut)? $arg:ident: $ty:ty )* );
    ) => {
        $pub fn $newname( $args );
    }
}
```

_TODO: as pointed out in the comments, this syntax is ambivuous between `$group
()` (recreate the group, add `()` after ) and `$group()` (expand the group).
`$...group` was proposed as an alterantive._

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Macro captures currently include the following grammar node:

> `$` ( _MacroMatch<sup>+</sup>_ ) _MacroRepSep_<sup>?</sup> _MacroRepOp_

This will be expanded to the following:

> `$` ( IDENTIFIER_OR_KEYWORD except crate | RAW_IDENTIFIER | _ )<sup>?</sup> ( _MacroMatch<sup>+</sup>_ ) _MacroRepSep_<sup>?</sup> _MacroRepOp_

As a result, `$identifier( /* ... */ ) /* sep and kleene */` will allow naming
a capture group. It can then be used in expansion:

```rust
$identifier(
  /* expansion within group */
) /* sep and kleene */
```

Group and metavariables share the same namespace; all groups and metavariables
must have a unique name within the macro capture pattern.

Names will remain optional; however, if a capture group is given a name, it
_must_ also be referred to by name during expansion. That is, an unnamed
group in expansion will never be matched to a named group in the pattern.

To make expansion rules easier, _it is forbidden to mix named and unnamed
groups_ within the same macro.

## Overview of changes

A summary of the implications of this language addition is provided before
explaining detailed semantics.

### Nesting repetition expansions

Nesting or intermixing repetition groups is currently not possible, mostly due
to ambiguity of capture group expansions. Using an example from above:

```rust
macro_rules! make_functions {
    (
    //           ↓ group 1
        names: [ $($name:ident),+ ],
    //  ↓ group 2
        $( greeting: $greeting:literal, )?
    ) => {
        $(  // <- this expansion contains both `$name` and `$greeting`. So is this
            //    an expansion of capture group 1 or 2?
            fn $name() {
                println!("function {} called", stringify!($name));
                $( println!("{}", $greeting) )?
            }
        )+
    }
}
```

Adding named capture groups makes this work, since ambiguity is removed.

It is likely possible to adjust the rules for expansion such that the above
would work with no additional syntax. However, this RFC posits that referring
to groups by name provides an overall better user experience than changing
the rules (more clear code, better diagnostics, and an easier model to follow).

### Zero-length capture groups

As a side effect of more precise repetition, groups in expansion that do not
contain any metavariables will become more straightforward. For example, this
simple counter is not possible as written:

```rust
macro_rules! count {
    ( $( $i:ident ),* ) => {{
        // Error: attempted to repeat an expression containing no syntax variables
        //↓  the compiler does not know which group this refers to (here there
        //   is only one choice, but that is not always the case).
        0 $( + 1 )*
    }};
}
```

Using named groups removes ambiguity so should work:

```rust
// Note: this is just a simple example. Metavariable expressions will provide a
// better way to get the same result with `${count(...)}`.
macro_rules! count {
    ( $idents( $i:ident ),* ) => {{
        0 $idents( + 1 )*
    }};
}
```

Metavariable expressions provide an `${ignore($var)}` operation that enables
the same behavior; `ignore(...)` will simply not be needed with named groups.

There is also no way to act on capture groups that bind only exact tokens but
no variables. An example is extracting the `mut` from a function or binding
signature:


```rust
/// Sample macro that captures exact syntax and tweaks it
macro_rules! match_fn {
    //               ↓ We need to be aware of mutability
    (fn $name:ident ($(mut)? $a:ident: u32)) => {
        //       ↓ we want to reproduce the `mut` here
        fn $name($(mut)? $a: u32) {
        //       ^^^^^^^
        // Error: attempted to repeat an expression containing no syntax variables
            println!("hello {}!", $a);
        }
    }
}
fn main() {
    match_fn!(fn foo(a: u32));
    foo(10);
}
```

Adding named capture groups to the above would allow it to work.
`${ignore(...)}` does not directly help here.

### Metavariable expressions

Metavariable expressions currently use a combination of location within the
expansion (i.e. which capture groups contain it), variables captured, and
an index to change the indicated group. For example, `index()` returns the
number of the current expansion.

```rust
macro_rules! innermost1 {
    ( $( $a:ident: $( $b:literal ),* );+ ) => {
        [$( $( ${ignore($b)} ${index(1)}, )* )+]
    };
}
```

In order to understand what `index(1)` is referring to here, one must do the
following:

- Note how many repetition groups exist in the match expression (2).
- Count how many repetition groups the `index(1)`` call is nested in (2).
- Backtrack by one to figure out what exactly is getting indexed (1).

After doing the above, it can be noted that `${index(1)}` in this position
will indicate the current expansion of the outer cature group (the group
containing only `$a`).

Rewritten to use named groups instead:

```rust
macro_rules! innermost1 {
    ( $outer_rep( $a:ident: $inner_rep( $b:literal ),* );+ ) => {
        [$outer_rep( $inner_rep( ${index($outer_rep)}, )* )+]
    };
}
```

It is significantly easier to see what the call to `index` is referring to. As
an added benefit, its meaning will not change if its position is moved in the
code (e.g. moving to be within `$outer_rep`, but not `$inner_rep`).

This RFC proposes that `count`, `index`, and `len` will accept group names
in place of a variable and an index, since these three expressions relate more
to how entire _groups_ are expanded than the variables they take as arguments.

Further reading:

- [`macro_metavar_expr` RFC][`macro_metavar_expr`] and
  [tracking issue](https://github.com/rust-lang/rust/issues/83527)
- [Proposal for possible specific behavior](https://github.com/rust-lang/rust/pull/122808#issuecomment-2124471027)

### "Exactly one" matching and full group emission

If a group is specified without a kleene operator (`*`, `+`, `?`), it will now
be assumed to match exactly once. This will be most useful with the ability to
emit an entire matched group.

```rust
macro_rules! check_and_pass {
    // `group1` will get matched exactly once
    ($group1(a b $v:ident c $group2($tt:tt)* )) => {
        // All tokens including exact `a` `b` will get passed to `other_macro`
        other_macro!($group1)
    }
}
```

This should make it much easier to work with optional exact matches. Currently
there is no way to do anything useful with capture groups that don't capture
metavariables (such as `$pub` and `$mut` in the
[Guide-level explanation](#guide-level-explanation) example).

_TODO: will this preserve the coercion of tokens to fragments
(e.g. tt -> ident)?_

## Detailed semantics

To illustrate detailed semantics, an example will be used in which the
token pattern approximately mirrors the named groups:

```rust
macro_rules! m {
    ( $outer_a(oa[ $middle(m[ $inner(i[ $x:ident ])* ])* ])*; $outer_b(ob[ $y:ident ])*)
        => { println!("{}", stringify!(/* relevant expansion */)) }
}

m!( oa[ m[i[x0] i[x1]] ] oa[] oa[m[] m[]]; ob[y0] );
```

This can be thought of as a tree structure that loosely matches the token
`Group` of the captured items.


```text
    Level 0     |    Level 1     |    Level 2     |     Level 3     |    Level 4

                                                  /- $inner[0,0,0] --- $x[0,0,0,0]
                |-- $outer_a[0] --- $middle[0,0] -|   `i[v0]`           `x0`
                |    `oa[...]`        `m[..]`     |
                |                                 \- $inner[0,0,1] --- $x[0,0,0,1]
                |                                     `i[v1]`           `x1`
                |
                |-- $outer_a[1]
                |    `oa[...]`
    $root      -|
(entire macro)  |                /- $middle[2,0]
                |-- $outer_a[2] -|   `m[..]`
                |    `oa[...]`   |
                |                \- $middle[2,1]
                |                    `m[..]`
                |
                |-- $outer_b[0] --- $y[0,0,0,1]
                    `ob[...]`        `y0`

Summary of captures:

- `$outer_a`: captured 3 times
- `$middle`: captured 3 times
- `$inner`: captured 2 times
- `$x`: captured 2 times
- `$outer_b`: captured 1 time
- `$y`: captured 1 time
```

In the above diagram, `$metavar[i_n, ..., i_1, i_0]` shows that this is the
`i`th


the `i`th
capture for the `n`th ancestor
captured instance of `$metavar`


 within the `i_1`th instance of its parent group
capture.

Some of this section intends to solidify rules that are currently implemented
but not well described.

### Definitions

This section uses some common terms to refer to relevant ideas.

- "Pattern" or "capture pattern": the left hand side of the macro that defines
  metavariables and gets pattern matched against code.
- "Captured": when a token or tokens are matched by a variable or group.
  This has some overlap with what rustc refers to as "repeating n times" in
  error messages, but this RFC seeks to make this less ambiguous.
- "Expansion": the right hand side of the macro that uses metavariables and new
  tokens to update the file's AST.
- "Contents": Whatever is contained within a group
- "Level": a nesting level (or generation in the tree diagram. Adding a new
  group around a metavarible increases its nesting level.
- "Parent": any group that is at a higher level of the subject ("child") and
  shares a direct lineage. E.g. `$inner` and `$middle` are both parents of `$v`.
- "Immediate parent": parent exactly one level above. E.g. `$inner` is an
  immediate  parent of `$v`, `$middle` is not.
- "Capture parent", "capture level", "capture contents": a parent, level, or
  group contents in the capture, which may not exist or be the same in the
  expansion.
- "Expansion parent", "expansion level", "expansion contents": a parent, level,
  or group contents in the expansion, which may not exist or be the same in the
  capture.

### Expansion rules

In current macro expansion, the following rules are observed:

1. Groups must contain at least one metavariable (`$()*` fails with "repetition
   matches empty token tree").
2. Metavariables must be expanded with the same level of nesting in which they
   are captured. That is, if a metavariable is captured within two nested
   groups as `$($($v:ident),*),*`, it may only be expanded as `$($($v)*)*`.
3. Metavariables _or groups_ at the same level of the capture group must be
   be captured the same number of times.

This is easier to visualize with examples. Named capture groups are used to
make things more clear, even though this is discussing the current unnamed
groups.

```rust
// Possible expansions for the above sample macro

// Ok: prints `x0 x1`
$outer_a( $middle( $inner( $x )* )* )*

// Ok: prints `y0`
$outer_b( $y )*

// Forbidden: "variable 'x' is still repeating at this depth"
$??( $x )*
$x

// Forbidden: "attempted to repeat an expression containing no syntax variables
// matched as repeating at this depth"
// Basically, it is unable to determine what the groups are supposed to refer to.
$outer_b( $??( $??( $y )* )* )*

// Forbidden: "`x` repeats 3 times, but `y` repeats 1 time"
// This is an example diagnostic where referring to groups by their captures
// doesn't work well; `x` is actually only captured twice, but _`$middle`_ is
// captured three times.
$??( $??( $??( $x $y )* )* )*

// Forbidden: "`x` repeats 3 times, but `y` repeats 1 time"
// Makes sense; if `$combined` must refer to only a single group then it has
// no way to pick between `$outer_a` and `$outer_b`.
$combined( $middle( $inner( $x )* )* $y )*

// ...except the above actually works with the following invocation, printing
// `x0 y0`, because `$middle` (level 2) and `$y` (also level 2) are captured
// the same number of times. This is an example of invocation-dependent
// expansion correctness that this RFC hopes to minimize.
m!( oa[ m[i[x0]] ]; ob[y0] );
```

With named repetition groups, these rules will be changed to the following:

1. Group expansions no longer need to contain any metavariables.
2. In expansion, the group will repeat as many times as its entire pattern
   was captured, independent of whatever its expansion contents are.
3. Captured variables or groups may only be expanded within their parent group,
   if any. However, the capture parent does not need to be the immediate
   expansion parent.
5. If a group name is given in the expansion with no `(...)`, the entire
   capture of that group is reemitted _including exact literals_.

These are detailed in the following sections.

#### Expansion within an immediate parent

If a group or metavariable has capture parents, it must be scoped within those
same parents for expansion (though they need not be immediate).

```rust
// Correct: prints `x0 x1`
$outer_a( $middle( $inner( $x )* )* )*

// Correct, emits entire middle group. Result: `m[i[x0] i[x1]] m[] m[]`
$outer_a( $middle )*

// Skipping a group level
// Error: `$inner` must be contained within a `$middle` group, but it is within
// `$outer_a`
$outer_a( $inner( $x )* )*

// No grouping at all
// Error: `$x` must be within an `$inner` group, but it is not within any group
$x
```

_TODO: if `foo[x0] foo[] foo[x2]` is captured and the expansion is
`$foo( $x ),+`, should `x0, x2` be emitted of `x0, , x2` (extra comma)?
Probably the first one._

A possible relaxation of this rule is to allow groups or variables to expand to
all captures within that level when not nested within the immediate parent.

TBD whether this should be part of this RFC or a future possibility

```rust
// Expands to `x0 x1`
$x

// Expands to `x0 x1`
$outer_a( $middle( $x )* )*
```

#### Expansion within non-parent groups

If a group or metavariable is nested within a group that is not a capture
parent, it should be repeated as many times as that group. It must still have
its capture parents as expansion parents so as not to break other rules, but
they need not be the immediate parents.

In order to avoid edge cases with metavariable expressions, a group is not
allowed to be nested within itself.

```rust
// Correct: prints `y0 y0 y0`
// This is because the _entire_ expansion of `$outer_b` (one instance of $y)
// is repeated once for each `$outer_a` (three instances)
$outer_a( $outer_b( $y ) )

// Correct: prints `ob[ oa[y0 i[x0 y0] oa[x1 y0]] oa[y0] oa[y0]]`.
// Explanation:
// - `root > outer_a > middle > inner > x` and `outer_b > y` ordering are both
//   still respected, even though they are interleaved
// - `outer_b` repeats once within root
// - `outer_a` repeats three times within root, so repeats 3x within `outer_b`
// - `inner` repeats `[2, 0, 0]` times within the `outer_a` instances. This
//   drives how often `x` and `y` get repeated within that group.
$outer_b( ob[$outer_a( oa[$y $middle( $inner( i[$x $y] ) )] )] )

// Forbidden: `x` is missing parent `outer_a`
$outer_b( $middle( $inner( $x $y ) ) ) )

// Forbidden: group nesting within itself
$outer_b( $outer_b( $y ) )
$outer_b( $outer_b )
```

#### Single matches

Capture groups must currently specify a kleene operator (`*`, `+`, or `?`) that
determines if the group should match zero or more times, one or more times, or
up to one time. This RFC will allow omitting the kleene operator to indicate
that a group must be captured exactly once. That is, `$group(foo)` is a valid
pattern (with current rules, `$(foo)` is forbidden).

With this "exactly once" match there is no purpose in having a repetition token
(e.g. the comma in `$(...),*`), so it must be omitted.

#### Entire group expansion

Since groups are named, it is now possible to write a group name to reemit its
captured contents with no further expansion. This syntax uses the group name
but omits the `(/* expansion pattern */)/* kleene */`:

```rust
// Ok: prints `ob[y0]`
$outer_b
```

The entire contents of the capture group are emitted, including both exact
tokens and anything that would be bound to a metavariable. Span from the
macro invocation can be kept here, which should improve the diagnosability of
some macros.

The above rules regarding allowed group usage locations must still be followed.

### Metavariable Expressions

This RFC proposes some changes to metavariable expressions that will leverage
named groups to hopefully make them more user-friendly.

_At time of writing, part of macro metavariable expressions are under
consideration for stabilization. Depending on what is selected, these rules may
need to change slightly._

- `${index($group)}`: Return the number of times that the group has been
  _expanded_ so far. Must be used within the group that is given as an
  argument.
- `${count($metavar)}`: Return the number of times a group or metavariable was
  _captured_.
- `${len($metavar)}`: Because `count` becomes more flexible, `len` is no longer
  needed and can be removed.
- `${ignore($metavar)}`: if this RFC is accepted then `ignore` can be removed.
  It is used to specify which capture group an expansion group belongs to when
  no metavariables are used in the expansion; with named groups, however, this
  is specified by the group name rather than by contained metavariables.

#### `${index($group)}`

The `index` metavariable expression is used to indicate the number of times
a group has been expanded so far. It can be thought of a form of `enumerate`.

- Arguments: one required argument, `$group`
- Allowed usage: may only be used within `$group`
- Output: The number of times the _current expansion of `$group`_ has repeated
  so far. That is, if `$group` is captured twice but used >2 times in the
  expansion, `${index($group)}` will still only ever return 0 or 1.
- Changes from current implementation:
  - Takes a group as an argument, not a depth
  - The argument is no longer optional

In the tree diagram, this can be thought of as returning the final number for
the given group in the `[i_n, ..., i_0]` index list.

```rust
// Ok: prints `o0 m0 i0 i1; o1; o2 m0 m1;`
$outer_a( o ${index($outer_a)} $middle( m ${index($middle)} $inner( i ${index($inner)})* ),* );*

// Ok: prints `o m outer_idx 0; o; o m outer_idx 2 m outer_idx 2`
$outer_a( o $middle( m outer_idx ${index($outer_a)} ),* );*

// Ok: prints `ob oa0 oa1 oa2`
// The outer repetition (`outer_b`) has no influence on `index`
$outer_b( ob $outer_a( oa ${index($outer_a)} ),* )*

// Forbidden: not used within a `$middle` group
$outer_a( ${index($middle)} $middle );*
```

The location of `index` within its group does not affect its output. That is,
all of the below will return 0:

```rust
// For this example, `$g1` is captured exactly once. All other groups are
// captured any number of times

// Prints `0`
$g1( ${index($g1)} )

// Increasing the nestin does nothing; still returns `0` for each `g2` capture
$g1( $g2( ${index($g1)} )* )

// Still returns `0`
$g1( $g2( $g3 ( ${index($g1)} )* )* )
```

#### `${count($name)}`

`count` is used to return the number of times a group or variable has been
captured. It can be used in any location, but its exact behavior is location-
dependent.

- Arguments: one required argument, `$group` or `$metavariable`
- Allowed usage: may be used anywhere within the expansion, but some arguments
  may be disallowed.
- Output: this returns the number of times a group or metavariable was
  captured, with some scoping specifics.
- Changes from current implementation:
  - Can take a group as an argument
  - Functionality combined with `len`

Looking at a group or variable that is more deeply nested will return how many
of that variable were captured in the current repetition. Looking at a variable
or group that is less deeply nested will return the total times that group was
captured.

This can be represented as a simple tree walking algorithm to the _capture_
tree to determine what gets counted. The starting position in the _expansion_
determines where to start, and then the following rules are applied:

- If `level($name)` >= `level(expression)` (more deeply nested), walk all
  descendents, including those of neighbors, and count each `$name`
- If `level($name) < `level(expression)` (less deeply nested):
  - Walk the entire tree and count each `$name`
  - Reject code with an error if `$name` is not an ancestor

_TODO: this was meant to be compatible with the existing metavariable
expressions, but after talking to Josh, this should probably be split to
two separate MVEs. Maybe something along the lines of `count_parents` and
`count_children` could work._

```rust
/* looking at descendents */

// Ok: prints `[oa 3, m 3, i 2, x 2; ob 1, y 1]`
// Demo printing totals. Expansion is at the root (level 0), so each variable
// is at a higher level; the entire tree is walked and all instances are counted.
[
    oa ${count($outer_a)}, m ${count($middle)}, i ${count($inner)}, x ${count($x)};
    ob ${count($outer_b)}, y ${count($y)},
]

// Ok: prints `[1 0 2]`
[$outer_a( ${count($middle} )*]

// Ok: prints `[2 0 0]`
[$outer_a( ${count($inner} )*]

// Ok: prints `[2 0 0]`
[$outer_a( ${count($x} )*]

// Ok: prints `o[m[x 2]] o[] o[m[x 0] m[x 0]]`
// `$middle[0,0]``"sees" two `$x` captures. `$middle[2,0]` and `$middle[2,1]` both
// don't see any.
$outer_a( o[$middle( m[x ${count($x)}] )*] )*

/* looking at ancestors */

// Ok: prints [3 3 3]
// `count` is used at level 2 (one deeper than `outer_a`), so it sees all
// `outer_a`s each time.
[$outer_a( ${count($outer_a)} )*]

// Ok: prints [[3] [] [3 3]]
// Similar to the above
[$outer_a( [$middle( ${count($outer_a)} )*] )*]

/* errors */

// Error: trying to count a variable that is neither a descendent or anscestor
[$outer_a( ${count($y} )*]
```

TODO: we could relax the final rule and allow counting siblings of ancestors

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

- If [`macro_metavar_expr`] stabilizes before this merges, this will add a
  duplicate way of using those expressions. If this RFC is accepted,
  stabilizing only a subset of metavariable expressions that does not conflict
  should be considered.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- In macro metavariable syntax, using named capture groups, we could treat `count` and `index` as *fields* rather than *functions*. For instance, we could write `${$group.index}` rather than `${index($group)}`. This would be consistent with the [macro fragment fields](https://github.com/rust-lang/rfcs/pull/3714) proposal.
- Since we no longer need both `count` and `len`, we could choose to use either name for the remaining function we still provide. We should consider whether `count` or `len` best describes this functionality.
- Variable definition syntax could be `$ident:(/* ... */)` rather than
  `$ident(/* ... */)`. Including the `:` is proposed to be more consistent
  with existing fragment specifiers.
- There is room for macros to become smarter in their expansions without adding
  named capture groups. As mentioned elsewhere in this RFC, it seems like
  adding named groups is a cleaner solution with less cognitive overhead.

# Prior art
[prior-art]: #prior-art

- Regex allows the naming of reepeating capture groups for expansion, including
  those that do not capture anything else.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Syntax: the original proposal was to include a colon, e.g.
  `$group1:(/* ... */)`. A label-like syntax of `$'group1 $(/* ... */)` was
  also proposed.

**Possibly edition-sensitive** the proposed syntaxes are currently rejected
under the `missing_fragment_specifier` lint. That means that
`#![allow(missing_fragment_specifier)]` makes rustc accept the proposed syntax
as valid, which could conflict with this proposal.

# Future possibilities
[future-possibilities]: #future-possibilities

- Macros 2.0: if accepted, the same rules expressed in this RFC should also
  apply to Macros 2.0. Macros 2.0 may even opt to forbid unnamed capture
  groups.
- A `${count_in($var, $group)}` expression that allows further scoping of
  `count` (TODO: describe this better).

[original proposal]: https://github.com/rust-lang/rfcs/pull/3649#discussion_r1618998153
[`macro_metavar_expr`]: https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html
