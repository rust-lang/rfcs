- Start Date: 2014-12-21
- RFC PR: [550](https://github.com/rust-lang/rfcs/pull/550)
- Rust Issue: [20563](https://github.com/rust-lang/rust/pull/20563)

# Key Terminology

- `macro`: anything invokable as `foo!(...)` in source code.
- `MBE`: macro-by-example, a macro defined by `macro_rules`.
- `matcher`: the left-hand-side of a rule in a `macro_rules` invocation.
- `macro parser`: the bit of code in the Rust parser that will parse the input
  using a grammar derived from all of the matchers.
- `NT`: non-terminal, the various "meta-variables" that can appear in a matcher.
- `fragment`: The piece of Rust syntax that an NT can accept.
- `fragment specifier`: The identifier in an NT that specifies which fragment
  the NT accepts.
- `language`: a context-free language.

Example:

```rust
macro_rules! i_am_an_mbe {
    (start $foo:expr end) => ($foo)
}
```

`(start $foo:expr end)` is a matcher, `$foo` is an NT with `expr` as its
fragment specifier.

# Summary

Future-proof the allowed forms that input to an MBE can take by requiring
certain delimiters following NTs in a matcher. In the future, it will be
possible to lift these restrictions backwards compatibly if desired.

# Motivation

In current Rust, the `macro_rules` parser is very liberal in what it accepts
in a matcher. This can cause problems, because it is possible to write an
MBE which corresponds to an ambiguous grammar. When an MBE is invoked, if the
macro parser encounters an ambiguity while parsing, it will bail out with a
"local ambiguity" error. As an example for this, take the following MBE:

```rust
macro_rules! foo {
    ($($foo:expr)* $bar:block) => (/*...*/)
};
```

Attempts to invoke this MBE will never succeed, because the macro parser
will always emit an ambiguity error rather than make a choice when presented
an ambiguity. In particular, it needs to decide when to stop accepting
expressions for `foo` and look for a block for `bar` (noting that blocks are
valid expressions). Situations like this are inherent to the macro system. On
the other hand, it's possible to write an unambiguous matcher that becomes
ambiguous due to changes in the syntax for the various fragments. As a
concrete example:

```rust
macro_rules! bar {
    ($in:ty ( $($arg:ident)*, ) -> $out:ty;) => (/*...*/)
};
```

When the type syntax was extended to include the unboxed closure traits,
an input such as `FnMut(i8, u8) -> i8;` became ambiguous. The goal of this
proposal is to prevent such scenarios in the future by requiring certain
"delimiter tokens" after an NT. When extending Rust's syntax in the future,
ambiguity need only be considered when combined with these sets of delimiters,
rather than any possible arbitrary matcher.

# Detailed design

The algorithm for recognizing valid matchers `M` follows. Note that a matcher
is merely a token tree. A "simple NT" is an NT without repetitions.
A "complex NT" is an NT that is not simple.  That is,
`$foo:ty` is a simple NT but `$($foo:ty)+` is not. `FOLLOW(NT)` is the set of
allowed tokens for the given NT's fragment specifier, and is defined below.

`CHECK(M, P):`  
  `M`: sequence of tokens comprising the matcher   
  `P`: set of successor tokens that may follow `M` "on the level above"  
  *output*: whether M is valid.
  1. If `M` is empty, accept.
  2. Set `t = HEAD(M)`
  3. If `t` is not an NT, skip to 7.
  4. Find `S`, the set of possible successors of `t`:
    1. `S = FIRST(TAIL(M))`
    2. If `ε` is in `S`, `S = S - {ε} + P`.  
       In other words, if the rest of `M` could match an empty string, we should 
       also consider the set of successors `P`.
  5. If `t` is a simple NT, check that `S` is a subset of `FOLLOW(t)`.
     If so, skip to 7, else, reject.
  6. Else, `t` is a complex NT.
      1. If `t` has the form `$(Q)+` or `$(Q)*`, run `CHECK(Q, FIRST(Q) + S)`.
         If it accepts, skip to 7, else, reject.
      2. If `t` has the form `$(Q)u+` or `$(Q)u*` for some token `u`,
         run `CHECK(Q, {u} + S)`. If it accepts, skip to 7, else, reject.
  7. Set `M = TAIL(M)`, goto 1.

`FIRST(M):` Returns the set of all possible tokens that may begin input sequence matched by `M`.
  1. If `M` is empty, return `{ε}`.
  2. Set `t = HEAD(M)`
  3. If `t` is not a complex NT, return `{t}`.
  4. If `t` is a complex NT:
     1. If `t` has the form `$(Q)+` or `$(Q)u+`, return `FIRST(Q)`.
     2. If `t` has the form `$(Q)*` or `$(Q)u*`, return `FIRST(Q) + FIRST(TAIL(M))`.

`HEAD(M)` Returns the first token in sequence `M`.   
`TAIL(M)` Returns sequence `M` with first token removed.  
(both `HEAD` and `TAIL` are not defined if `M` is empty)

This algorithm should be run on every matcher in every `macro_rules`
invocation, with `P` as `{}` (empty set). If it rejects a matcher, an error should be
emitted and compilation should not complete.

The current legal fragment specifiers are: `item`, `block`, `stmt`, `pat`,
`expr`, `ty`, `ident`, `path`, `meta`, and `tt`.

- `FOLLOW(pat)` = `{FatArrow, Comma, Eq}`
- `FOLLOW(expr)` = `{FatArrow, Comma, Semicolon}`
- `FOLLOW(ty)` = `{Comma, FatArrow, Colon, Semicolon, Eq, Gt, Ident(as)}`
- `FOLLOW(stmt)` = `FOLLOW(expr)`
- `FOLLOW(path)` = `FOLLOW(ty)`
- `FOLLOW(block)` = any token
- `FOLLOW(ident)` = any token
- `FOLLOW(tt)` = any token
- `FOLLOW(item)` = any token
- `FOLLOW(meta)` = any token

##### Example 1
CHECK(M = (`$a:expr` `$b:expr`), P = {})

1. M is not empty
2. t = `$a:expr`
3. t is NT
4. S = FIRST(TAIL(`$a:expr` `$b:expr`))  
     = { `$b:expr` }
5. S not in FOLLOW(`expr`) => REJECT  

##### Example 2
CHECK(M = (`$a:expr` `$( : $b:expr),*` `$c:expr`), P = {})

1. M is not empty
2. t = `$a:expr`
3. t is NT
4. S = FIRST(TAIL(`$a:expr` `$( ; $b:expr),*` `$c:expr`))  
     = FIRST( `$(; $b:expr),*` `$c:expr` )  
     = FIRST( `;` `$b:expr` ) + FIRST(`$c:expr`) [by rule FIRST.4.2]  
     = { `;` `$c:expr` }
5. S not in FOLLOW(`expr`) because of `$c:expr` => REJECT

##### Example 3

CHECK(M = (`$($a:expr)*`), P = {})

1. M is not empty
2. t = `$($a:expr)*`
3. t is NT
4.  
  1. S = FIRST(TAIL(`$($a:expr)*`)) = FIRST() = {`ε`}
  2. S = S - {`ε`} + P = {}
5. t is not a simple NT
6. CHECK( `$a:expr`, FIRST(`$a:expr`) + {}) [by rule CHECK.6.1]
   
nested CHECK(M = (`$a:expr`), P = {`$a:expr`})

1. M is not empty
2. t = `$a:expr`
3. t is NT
4.  
  1. S = FIRST(TAIL(`$a:expr`))  
     = FIRST()
     = {`ε`}
  2.  S = S - {`ε`} + P = { `$a:expr` }
5. t is a simple NT, S not in FOLLOW(`expr`) => REJECT


# Drawbacks

It does restrict the input to a MBE, but the choice of delimiters provides
reasonable freedom and can be extended in the future.

# Alternatives

1. Fix the syntax that a fragment can parse. This would create a situation
   where a future MBE might not be able to accept certain inputs because the
   input uses newer features than the fragment that was fixed at 1.0. For
   example, in the `bar` MBE above, if the `ty` fragment was fixed before the
   unboxed closure sugar was introduced, the MBE would not be able to accept
   such a type. While this approach is feasible, it would cause unnecessary
   confusion for future users of MBEs when they can't put certain perfectly
   valid Rust code in the input to an MBE. Versioned fragments could avoid
   this problem but only for new code.
2. Keep `macro_rules` unstable. Given the great syntactical abstraction that
   `macro_rules` provides, it would be a shame for it to be unusable in a
   release version of Rust. If ever `macro_rules` were to be stabilized, this
   same issue would come up.
3. Do nothing. This is very dangerous, and has the potential to essentially
   freeze Rust's syntax for fear of accidentally breaking a macro.
