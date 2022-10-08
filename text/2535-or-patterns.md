- Feature Name: `or_patterns`
- Start Date: 2018-08-29
- RFC PR: [rust-lang/rfcs#2535](https://github.com/rust-lang/rfcs/pull/2535)
- Rust Issue: [rust-lang/rust#54883](https://github.com/rust-lang/rust/issues/54883)

# Summary
[summary]: #summary

Allow `|` to be arbitrarily nested within a pattern such
that `Some(A(0) | B(1 | 2))` becomes a valid pattern.

# Motivation
[motivation]: #motivation

Nothing this RFC proposes adds anything with respect to expressive power.
Instead, the aim is to make the power we already have more easy to wield.
For example, we wish to improve ergonomics, readability, and the mental model.

## Don't repeat yourself

Consider an example match arm such as (1):

```rust
Some(Enum::A) | Some(Enum::B) | Some(Enum::C) | Some(Enum::D) => ..
```

Here, we are repeating `Some($pat)` three times.

Compare (1) to how we could have written this with this RFC (2):

```rust
Some(Enum::A | Enum::B | Enum::C | Enum::D) => ..
```

We can see that this is clearly shorter and that the amount of extra work
we have to do scales linearly with the number of inner variants we mention.
The ability to nest patterns in this way therefore results in improved
writing ergonomics.

## Mental model

However, as we know, code is read more than it is written. So are we trading
readability for increased ergonomics? We believe this is not the case.
Instead, this RFC aims to improve the readability of code by reducing the
amount of redundant information that needs to be scanned.

In addition, we aim to more closely align Rust with the mental model that
*humans* have and how we usually speak and communicate.

Consider that you wanted to ask someone what the *colour* of their *car* was.
Would you be more inclined to ask:

> Is your car red, white, or blue?

Or would you instead ask:

> Is your car red, your car white, or your car blue?

[CNF]: https://en.wikipedia.org/wiki/Conjunctive_normal_form

[DNF]: https://en.wikipedia.org/wiki/Disjunctive_normal_form

When researching for this RFC; many people were asked and all of them preferred
the first alternative. This user testing was done on both programmers and
non-programmers alike and included speakers of: English, German (2) Swedish (3),
French (2), Portuguese (1), Spanish (2), Farsi (3), Finnish (1), Esperanto (1),
and Japanese (1).

Thus, we conjecture that it's more common for humans to not distribute and to
instead use something akin to *conjunctive normal form* ([CNF]) when communicating.
A likely consequence of this is that a common way to understand snippet (1)
formulated in *disjunctive normal form* ([DNF]) is to first mentally reconstruct
it into CNF and then understand the implications of the pattern.

By allowing users to encode their logic in the way they think instead of going
through more indirect routes, we can improve the understandability of code.

## Reducing complexity with uniformity

A principal way in which programming languages accumulate complexity is by
adding more and more rules that a programmer needs to keep in their head to
write or understand what a program does. A consequence of this is that often
times, caveats and corner cases make for a language that is harder to learn,
understand, and write in. To avoid such caveats, it is thus imperative that
we should try to keep the language more uniform rather than less.
This is an important means through which it becomes possible to give users
more expressiveness but at the same time limit the cost each feature takes
from our complexity budget.

With this RFC, we try to reduce the complexity of the language by extending
a feature which already exists, and which many users already know about,
to another place. In a sense, giving the user more capabilities results
in a negative increase in complexity.

[RFC 2175]: https://github.com/rust-lang/rfcs/pull/2175

In concrete terms, where before we only allowed a pattern of the form
`pat | pat` at the top level of `match` and [similar constructs][RFC 2175],
which special cased the language, we now allow `pat | pat` anywhere a pattern
may occur whereby we simplify the ruleset of the language.
In fact, there are already users that try this expecting it to work but
then find out that it does not.

Furthermore, allowing `pat | pat` in the pattern grammar also allows macros to
produce disjunctions such as `$p | $q`.

## Real world use cases

This RFC wouldn't be complete without concrete use cases which it would
facilitate. While there are not an overabundance of cases where `pat | pat`
would help, there are some where it would. Let's go through a few of them.

[precursor]: https://github.com/rust-lang/rfcs/blob/de235887a80555427314c7eb25c6214523d50cce/text/0000-pipe-in-patterns.md

1. One example which was raised in the [precursor] to this RFC was building a
   state machine which is iterating through `chars_indices`:

   ```rust
   match iter.next() {
       Some(_, ' ' | '\n' | '\r' | '\u{21A1}') => {
           // Change state
       }
       Some(index, ch) => {
           // Look at char
       }
       None => return Err(Eof),
   }
   ```

[GHC proposal]: https://github.com/osa1/ghc-proposals/blob/77ee8e615aa28fbf2d0ef2be876a852c4e63c53b/proposals/0000-or-patterns.rst#real-world-examples

2. Other examples are listed in the equivalent [GHC proposal].

3. Another example which was provided in the [precursor] RFC was:

   ```rust
   for event in event_pump.poll_iter() {
       use sdl2::event::Event;
       use sdl2::keyboard::Keycode::{Escape, Q};
       match event {
           Event::KeyDown { keycode: Some(Escape | Q), ... } => break 'game,
           _ => {},
       }
       ...
   }
   ```

4. Other cases where this feature was requested include:
   + <https://github.com/rust-lang/rust/issues/15219>
   + <https://github.com/rust-lang/rust/issues/14516>

[alercah_discord]: https://discordapp.com/channels/442252698964721669/448237931136679936/483325957130813440

5. Another use case due to [@alercah][alercah_discord] is:

   ```rust
   pub fn is_green(self) -> bool {
       match self {
           | Tile::Suited(Suit::Souzu, 2 | 3 | 4 | 6 | 8)
           | Tile::Dragon(Dragon::Green) => true,
           _ => false,
       }
   }
   ```

6. Some further examples found with sourcegraph include:

   From [cc-rs](https://github.com/alexcrichton/cc-rs/blob/74ce606aa227a30a97d7c1990c1e8d322e01c6d8/src/lib.rs#L1307-L1319):

   ```rust
    match (self.cpp_set_stdlib.as_ref(), cmd.family) {
        (None, _) => {}
        (Some(stdlib), ToolFamily::Gnu | ToolFamily::Clang) => {
            cmd.push_cc_arg(format!("-stdlib=lib{}", stdlib).into());
        }
        _ => {
            ...
        }
    }
   ```

   From [capnproto](https://github.com/capnproto/capnproto-rust/blob/35027494bb6e741aa478597358bac8ac92108a30/capnp/src/private/layout.rs#L1979-L2002):

   ```rust
   // Check whether the size is compatible.
   match expected_element_size {
       None | Some(Void | InlineComposite) => (),
       Some(Bit) => { ... }
       Some(Byte | TwoBytes | FourBytes | EightBytes) => { ... },
       ...
   }
   ```

   From [chrono](https://github.com/chronotope/chrono/blob/94b43fa2e8bd43e7f42bb5b67afd1c3415b27683/src/format/parsed.rs#L271-L308):

   ```rust
   fn resolve_year(y: Option<i32>, q: Option<i32>,
                   r: Option<i32>) -> ParseResult<Option<i32>> {
       match (y, q, r) {
           (y, None, None) => Ok(y),
           (Some(y), q, r @ (Some(0...99) | None)) => { ... },
           ...
       }
   }
   ```

   From maidsafe's [routing](https://github.com/maidsafe/routing/blob/0081a48d59e4fe3fb86b20da1fceb8f757855112/src/states/node.rs#L2138-L2180):

   ```rust
   match self.peer_mgr.connection_info_received(...) {
       ...,
       Ok(IsProxy | IsClient | IsJoiningNode) => { ... },
       Ok(Waiting | IsConnected) | Err(_) => (),
   }
   ```

   Also from [routing](https://github.com/maidsafe/routing/blob/0081a48d59e4fe3fb86b20da1fceb8f757855112/src/states/node.rs#L2215-L2245):

   ```rust
        match self.peer_mgr.connection_info_received(...) {
            Ok(Ready(our_info, their_info)) => { ... }
            Ok(Prepare(_) | IsProxy | IsClient | IsJoiningNode) => { ... }
            Ok(Waiting | IsConnected) | Err(_) => (),
        }

   ```

   From [termion](https://github.com/redox-os/termion/blob/d2945cd36c452824aeabd5d7c13980d9567eb8a2/src/input.rs#L143-L153):

   ```rust
   for c in self.bytes() {
       match c {
           Err(e) => return Err(e),
           Ok(0 | 3 | 4) => return Ok(None),
           Ok(0x7f) => { buf.pop(); }
           Ok(b'\n' | b'\r') => break,
           Ok(c) => buf.push(c),
       }
   }
   ```

7. Some other use cases are:

   In code using git2-rs:

   ```rust
   match obj.kind() {
       Some(Commit | Tag | Tree) => ...
       Some(Blob) => ...
       None => ...
   }
   ```

   From [debcargo](https://salsa.debian.org/rust-team/debcargo/blob/4355097810264644cb08ddaa8f7464d5887275f1/src/debian/dependency.rs#L234-291):

   ```rust
   match (op, &mmp.clone()) {
       (&Lt, &(M(0) | MM(0, 0) | MMP(0, 0, 0))) => debcargo_bail!(
           "Unrepresentable dependency version predicate: {} {:?}",
           dep.name(),
           p
       ),
       (&Tilde, &(M(_) | MM(_, _))) => {
           vr.constrain_lt(mmp.inclast());
           vr.constrain_ge(mmp);
       }
       (&Compatible, &(MMP(0, minor, _) | MM(0, minor))) => {
           vr.constrain_lt(MM(0, minor + 1));
           vr.constrain_ge(mmp);
       }
       (&Compatible, &(MMP(major, _, _) | MM(major, _) | M(major))) => {
           vr.constrain_lt(M(major + 1));
           vr.constrain_ge(mmp);
       }
       ...,
   }
   ```

8. From rustc, we have:

   In `src/librustc_mir/interpret/eval_context.rs`:
   ```rust
   Some(Def::Static(..) | Def::Const(..) | Def::AssociatedConst(..)) => {},
   ```

   In `src/librustc_mir/util/borrowck_errors.rs`:
   ```rust
   (&ty::TyArray(_, _), Some(true) | None) => "array",
   ```

   In `src/librustc/middle/reachable.rs`:
   ```rust
   Some(Def::Local(node_id) | Def::Upvar(node_id, ..)) => { .. }
   ```

   In `src/librustc/infer/error_reporting/mod.rs`:
   ```rust
   Some(hir_map::NodeBlock(_) | hir_map::NodeExpr(_)) => "body",
   ```

   In `src/libfmt_macros/lib.rs`:
   ```rust
   Some((_, '>' | '<' | '^')) => { .. }
   ```

   In `src/librustc/traits/select.rs`:
   ```rust
   ty::TyInfer(ty::IntVar(_) | ty::FloatVar(_)) | .. => { .. }
   ```

   In `src/librustc_typeck/check/mod.rs`:
   ```rust
   ty::TyInt(ast::IntTy::I8 | ast::IntTy::I16) | ty::TyBool => { .. }

   ...

   ty::TyUint(ast::UintTy::U8 | ast::UintTy::U16) => { .. }
   ```

   In `src/tools/cargo/src/cargo/sources/path.rs`:
   ```rust
   Some("Cargo.lock" | "target") => continue,
   ```

   In `src/libsyntax_ext/format_foreign.rs`:
   ```rust   
   ('h' | 'l' | 'L' | 'z' | 'j' | 't' | 'q', _) => {
       state = Type;
       length = Some(at.slice_between(next).unwrap());
       move_to!(next);
   },

   ...

   let width = match self.width {
       Some(Num::Next) => {
           // NOTE: Rust doesn't support this.
           return None;
       }
       w @ Some(Num::Arg(_) | Num::Num(_)) => w,
       None => None,
   };
   ```

   In `src/libsyntax/parse/token.rs`:

   ```rust
   BinOp(Minus | Star | Or | And) | OrOr => true,
   ```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Simply put, `$p | $q` where `$p` and `$q` are some patterns is now itself
a legal pattern.

This means that you may for example write:

```rust
enum Foo<T> {
    Bar,
    Baz,
    Quux(T),
}

fn main() {
    match Some(Foo::Bar) {
        Some(Foo::Bar | Foo::Baz) => { .. },
        _ => { .. },
    }
}
```

Because `$p | $q` is itself a pattern, this means that you can nest arbitrarily:

```rust
fn main() {
    match Some(Foo::Bar) {
        Some(Foo::Bar | Foo::Quux(0 | 1 | 3)) => { .. },
        _ => { .. }
    }
}
```

Note that the operator `|` has a low precedence. This means that if you
want the same outcome as `foo @ 1 | foo @ 2 | foo @ 3`, you have to write
`foo @ (1 | 2 | 3)` instead of writing `foo @ 1 | 2 | 3`.
This is discussed in the [rationale][alternatives].

You can also use `p | q` in:

1. `if let` expressions:

   ```rust
   if let Foo::Bar | Foo::Quux(1 | 2) = some_computation() {
       ...
   }
   ```

1. `while let` expressions:

   ```rust
   while let Ok(1 | 2) | Err(3) = different_computation() {
       ...
   }
   ```

3. `let` statements:

   ```rust
   let Ok(x) | Err(x) = another_computation();
   ```

   In this case, the pattern must be irrefutable as `Ok(x) | Err(x)` is.

4. `fn` arguments:

   ```rust
   fn foo((Ok(x) | Err(x)): Result<u8, u8>) {
       ...
   }
   ```

   Here too, the pattern must be irrefutable.

5. closure arguments:

   ```rust
   let closure = |(Ok(x) | Err(x))| x + 1;
   ```

   Notice that in this case, we have to wrap the pattern in parenthesis.
   This restriction is currently enforced to avoid backtracking but may possibly
   be lifted in the future based on other developments in the grammar.

6. macros by example:

   ```rust
   macro_rules! foo {
       ($p:pat) => { ... }
   }

   foo!((Ok(x) | Err(x)));
   ```

   Here we must wrap the pattern in parenthesis since `$p:pat | $q:pat` is
   already legal in patterns.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

We parameterize the `pat` grammar by the choice whether to allow top level
`pat | pat`. We then change the pattern grammar to:

```rust
pat<allow_top_alt>
: pat<allow_top_alt> '|' pat<allow_top_alt>
| ...
;

pat<no_top_alt>
: "(" pat<allow_top_alt> ")"
| ...
;
```

Here `|` has the lowest precedence.
In particular, the operator `@` binds more tightly than `|` does.
Thus, `i @ p | q` associates as `(i @ p) | q` as opposed to `i @ (p | q)`.

Note: `pat<T>` does not entail that the grammar of Rust is context sensitive
because we "monomorphize" the parameterization below.

We then introduce a production:

```rust
top_pat : '|'? pat<allow_top_alt> ;
```

We then change the grammar of `let` statements to (as compared to [RFC 2175]):

```rust
let : LET top_pat maybe_ty_ascription maybe_init_expr ';' ;
```

We change the grammar of `if let` expressions to:

```rust
expr_if_let : IF LET top_pat '=' expr_nostruct block (ELSE block_or_if)? ;
```

And for `while let` expressions:

```rust
expr_while_let : maybe_label WHILE LET top_pat '=' expr_nostruct block ;
```

For `for` loop expressions we now have:

```rust
expr_for : maybe_label FOR top_pat IN expr_nostruct block ;
```

For `match` expressions we now have:

```rust
expr_match : MATCH expr_nostruct '{' match_clause* nonblock_match_clause? '}' ;
match_clause : nonblock_match_clause ',' | block_match_clause ','? ;
nonblock_match_clause : match_arm (nonblock_expr | block_expr_dot) ;
block_match_clause : match_arm (block | block_expr) ;

match_arm : maybe_outer_attrs top_pat (IF expr_nostruct)? FAT_ARROW ;
```

In other words, in all of the contexts where a pattern is currently accepted,
the compiler will now accept pattern alternations of form `p | q` where
`p` and `q` are arbitrary patterns.

For the patterns of `fn` arguments we now have:

```rust
param : pat<no_top_alt> ':' ty_sum ;
```

For closures we now have:

```rust
inferable_param : pat<no_top_alt> maybe_ty_ascription ;
```

Finally, `pat` macro fragment specifiers will also match the `pat<no_top_alt>`
production as opposed to `pat<allow_top_alt>`.

### Error messages

As previously noted, the precedence of the operator `|` is lower than that of
the operator `@`. This results in `i @ p | q` being interpreted as `(i @ p) | q`.
In turn, this would result in an error because `i` is not defined in all
alternations. An example:

```rust
fn main() {
    match 1 {
        i @ 0 | 1 => {},
    }
}
```

This would result in:

```rust
error[E0408]: variable `i` is not bound in all patterns
 --> src/main.rs:3:17
  |
3 |         i @ 0 | 1 => {},
  |         -       ^ pattern doesn't bind `i`
  |         |
  |         variable not in all patterns
```

However, it is quite likely that a user who wrote `i @ p | q` wanted the
semantics of `i @ (p | q)` because it would be the only thing that would
be a well formed pattern. To guide the user on the way, we recommend special
casing the error message for such circumstances with for example:

```rust
error[E0408]: variable `i` is not bound in all patterns
 --> src/main.rs:3:17
  |
3 |         i @ 0 | 1 => {},
  |         -       ^ pattern doesn't bind `i`
  |         |
  |         variable not in all patterns
  |
  | hint: if you wanted `i` to cover both cases, try adding parentheses around:
  |
  |         i @ 0 | 1
  |             ^^^^^
```

The particular design of such an error message is left open to implementations.

## Static semantics

1. Given a pattern `p | q` at some depth for some arbitrary patterns `p` and `q`,
   the pattern is considered ill-formed if:

   + the type inferred for `p` does not unify with the type inferred for `q`, or
   + the same set of bindings are not introduced in `p` and `q`, or
   + the type of any two bindings with the same name in `p` and `q` do not unify
     with respect to types or binding modes.

   [type coercions]: https://doc.rust-lang.org/reference/type-coercions.html

   Unification of types is in all instances aforementioned exact and
   implicit [type coercions] do not apply.

2. When type checking an expression `match e_s { a_1 => e_1, ... a_n => e_n }`,
   for each match arm `a_i` which contains a pattern of form `p_i | q_i`,
   the pattern `p_i | q_i` is considered ill formed if,
   at the depth `d` where it exists the fragment of `e_s` at depth `d`,
   the type of the expression fragment does not unify with `p_i | q_i`.

3. With respect to exhaustiveness checking, a pattern `p | q` is
   considered to cover `p` as well as `q`. For some constructor `c(x, ..)`
   the distributive law applies such that `c(p | q, ..rest)` covers the same
   set of value as `c(p, ..rest) | c(q, ..rest)` does. This can be applied
   recursively until there are no more nested patterns of form `p | q` other
   than those that exist at the top level.

   Note that by *"constructor"* we do not refer to tuple struct patterns,
   but rather we refer to a pattern for any product type.
   This includes enum variants, tuple structs, structs with named fields,
   arrays, tuples, and slices.

## Dynamic semantics

1. The dynamic semantics of pattern matching a scrutinee expression `e_s`
   against a pattern `c(p | q, ..rest)` at depth `d` where `c` is some constructor,
   `p` and `q` are arbitrary patterns, and `rest` is optionally any remaining
   potential factors in `c`, is defined as being the same as that of
   `c(p, ..rest) | c(q, ..rest)`.

## Implementation notes

With respect to both static and dynamic semantics,
it is always valid to first desugar a pattern `c(p | q)`
in CNF to its equivalent form in DNF, i.e. `c(p) | c(q)`.

However, implementing `c(p | q)` in terms of a pure desugaring to `c(p) | c(q)`
may not be optimal as the desugaring can result in multiplicative blow-up of patterns.
An example of such blow up can be seen with:

```rust
match expr {
    (0 | 1, 0 | 1, 0 | 1, 0 | 1) => { ... },
}
```

If we expanded this naively to DNF we would get:

```rust
match expr {
    | (0, 0, 0, 0)
    | (0, 0, 0, 1)
    | (0, 0, 1, 0)
    | (0, 0, 1, 1)
    | (0, 1, 0, 0)
    | (0, 1, 0, 1)
    | (0, 1, 1, 0)
    | (0, 1, 1, 1)
    | (1, 0, 0, 0)
    | (1, 0, 0, 1)
    | (1, 0, 1, 0)
    | (1, 0, 1, 1)
    | (1, 1, 0, 0)
    | (1, 1, 0, 1)
    | (1, 1, 1, 0)
    | (1, 1, 1, 1)
    => { ... },
}
```

Instead, it is more likely that a one-step case analysis will be more efficient.

Which implementation technique to use is left open to each Rust compiler.

# Drawbacks
[drawbacks]: #drawbacks

1. Some parsers will have to be rewritten by a tiny bit;
   We do this with any syntactic change in the language so
   there should not be any problem.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

As for why the change as proposed in this RFC should be done,
it is discussed in the [motivation].

## Syntax

Since we already use `|` for alternation at the top level, the only consistent
operator syntax for alternations in nested patterns would be `|`.
Therefore, there are not many design choices to make with respect to *how*
this change should be done rather than *if*.

## Precedence

With respect to the precedence of `|`, we cannot interpret `i @ p | q`
as `i @ (p | q)` because it is already legal to write `i @ p | j @ q`
at the top level of a pattern. Therefore, if we say that `|` binds more tightly,
then `i @ p | j @ q` will associate as `i @ (p | j @ q)` which as a different
meaning than what we currently have, thus causing a breaking change.

And even if we could associate `i @ p | q` as `i @ (p | q)` there is a good
reason why we should not. Simply put, we should understand `@` as a
pattern / set intersection operator and the operator `|` as the union operator.
This is analogous to multiplication and addition as well as conjunction and
disjunction in logic. In these fields, it is customary for multiplication and
conjunction to bind more tightly. That is, we interpret `a * b + c` as
`(a * b) + c` and not `a * (b + c)`. Similarly, we interpret `p ∧ q ∨ r`
as `(p ∧ q) ∨ r` and not `p ∧ (q ∨ r)`.

## Leading `|`

The only real choice that we do have to make is whether the new addition to the
pattern grammar should be `pat : .. | pat "|" pat ;` or if it instead should be
`pat : .. | "|"? pat "|" pat ;`. We have chosen the former for 4 reasons:

1. If we chose the former we can later change to the latter but not vice versa.
   This is thus the conservative choice.

2. There is precedent for such a decision due to [OCaml][ocaml].

3. The benefit to macros is dubious as they don't have to produce leading
   alternations.

4. Leading alternations inside patterns is considered poor style.

However, there is one notable advantage to permitting leading `|` in nested
pattern:

1. Libraries or tools such as `syn` will have *slightly* easier time parsing
   the grammar of Rust.

## `fn` arguments

In this RFC, we allow `p | q` inside patterns of `fn` arguments.
The rationale for this is simply consistency with `let` which also permit
these and did so before this RFC at the top level with [RFC 2175].

## Macros and closures

See the section on [unresolved] questions for a brief discussion.

# Prior art
[prior-art]: #prior-art

## CSS4 selectors

[CSS4]: https://drafts.csswg.org/selectors/#matches

In [CSS4] (draft proposal), it is possible to write a selector
`div > *:matches(ul, ol)` which is equivalent to `div > ul, div > ol`.
The moral equivalent of this in Rust would be: `Div(Ul | Ol)`.

## Regular expressions

[regex]: https://en.wikipedia.org/wiki/Regular_expression

Most [regular expression][regex] formalisms support at least the
following operations (where `a`, `b`, and `c` are arbitrary regexes):

+ Concatenation: *"`a` followed by `b`"*.
  Commonly written by just saying `ab`.

+ Alternation: *"first match `a` or otherwise match `b`"*
  Commonly written as `a | b`.
  `|` binds more loosely than concatenation.

+ Grouping: used to define the scope of what operators apply to.
  Commonly written as `(a)`.

Formally, the the minimal formalism we need is:

```rust
pat : terminal | pat pat | pat "|" pat | "(" pat ")" ;
```

Given this formalism, it is then possible to encode a regex:

```rust
a(b | c)
```

By the law of distributivity, we can rewrite this as:

```rust
ab | ac
```

## OCaml
[ocaml]: #ocaml

[ocaml_support]: https://caml.inria.fr/pub/docs/manual-ocaml/patterns.html#sec108

[This is supported][ocaml_support] in OCaml.
An example from "Real World OCaml" is:

```ocaml
let is_ocaml_source s =
  match String.rsplit2 s ~on:'.' with
  | Some (_, ("ml" | "mli")) -> true
  | _ -> false
```

While OCaml will permit the following:

```ocaml
let foo =
  match Some(1) with
  | Some(1 | 2) -> true
  | _ -> false
```

the OCaml compiler will reject:

```ocaml
let foo =
  match Some(1) with
  | Some(| 1 | 2) -> true (* Note in particular the leading | in Some(..). *)
  | _ -> false
```

We have chosen to impose the same restriction as OCaml here with respect to
not allowing leading `|` in nested pattern alternations.

## F#

[fsharp_patterns]: https://docs.microsoft.com/en-us/dotnet/fsharp/language-reference/pattern-matching

A language which is quite similar to OCaml is F#.
With respect to [pattern matching][fsharp_patterns], we may write:

```fsharp
let detectZeroOR point =
    match point with
    | (0, 0) | (0, _) | (_, 0) -> printfn "Zero found."
    | _ -> printfn "Both nonzero."
```

F# calls these "OR pattern"s and includes
`pattern1 | pattern2` in the pattern grammar.

## Haskell

[ghc_proposal_43]: https://github.com/ghc-proposals/ghc-proposals/pull/43

The [equivalent proposal][ghc_proposal_43] is currently being discussed for
inclusion in Haskell.

## Lisp

[lisp_libs]: https://stackoverflow.com/a/3798659/1063961

There is support for or-patterns in [various lisp libraries][lisp_libs].

# Unresolved questions
[unresolved]: #unresolved-questions

1. Should we allow `top_pat` or `pat<allow_top_alt>` in `inferable_param` such
   that closures permit `|Ok(x) | Err(x)|` without first wrapping in parenthesis?

   We defer this decision to stabilization as it may depend on experimentation.
   Our current inclination is to keep the RFC as-is because the ambiguity is not
   just for the compiler; for humans, it is likely also ambiguous and thus
   harder to read.

   This also applies to functions which, although do not look as ambiguous,
   benefit from better consistency with closures. With respect to function
   arguments there's also the issue that not disambiguating with parenthesis
   makes it less clear whether the type ascription applies to the or-pattern
   as a whole or just the last alternative.

2. Should the `pat` macro fragment specifier match `top_pat` in different
   Rust editions or should it match `pat<no_top_alt>` as currently specified?
   We defer such decisions to stabilization because it depends on the outcome
   of crater runs to see what the extent of the breakage would be.

The benefit of avoiding `pat<no_top_alt>` in as many places as possible would
both be grammatical consistency and fewer surprises for uses.
The drawbacks would be possible ambiguity or backtracking for closures and
breakage for macros.
