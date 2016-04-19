- Feature Name: `vis_matcher`
- Start Date: 2016-04-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `vis` matcher to `macro_rules!` that matches valid visibility annotations.

# Motivation
[motivation]: #motivation

Currently, matching visibility in a macro is something of a bug-bear.  Depending on the circumstances, there are two available approaches:

1. Match `$(pub)*`.  This may or may not be valid, depending on what follows this.  It also has the disadvantage of *not* allowing you to capture the visibility for use in substitution.  This is *only* useful if the visibility is irrelevant.

2. Write at least two rules, one for `pub` and one for non-`pub`.  I say "at least two" because, due to `macro_rules!` limitations, you may *already* be writing multiple rules to distinguish between other, unrelated syntax cases.

An additional frustration is in passing visibility information around within a macro invocation.  The simplest approach is use approach #2 above and pass either `()` or `(pub)` to sub-invocations.  This can be matched as a single `tt`, and "unpacked" at the other end using `($($vis:tt)*)`.  Unfortunately, due to limitations in `macro_rules!`, this requires the use of the "reparse trick" to get the resulting expansion to parse correctly.

Finally, due to this inconsistency, combined with the inability to use a capture *after* a repetition, it is impossible to parse (for example) a sequence of struct fields with visibility annotations.

All of this is to say: handling visibility in macros is, at present, a *discomfort in the posterior*.

The recently accepted [RFC #1422](https://github.com/rust-lang/rfcs/blob/master/text/1422-pub-restricted.md) has made the situation *worse*.

Now, there are three syntactically distinct variants of visibility: nothing, `pub`, and `pub(...)`.  As such, the approaches to handling this become:

1. Match `$(pub$(($($vis:tt)*))*)*`.  Hopefully, we can all agree that this is just *silly*.

2. Write at least *three* rules, one for `pub`, one for non-`pub`, and one for `pub($($vis:tt)*)`.

As a result of all of the above, I believe it is high time Rust gained a matcher for visibility annotations, in which case everything I just wrote goes away and instead becomes:

1. Match `$vis:vis`.

The exception is parsing a sequence of struct field which, due to attributes, remains impossible to do in general.  However, in cases where the existence of attributes can be ignored, it does *become* possible.

# Detailed design
[design]: #detailed-design

Introduce a new `macro_rules!` matcher kind called `vis`.  It should call `Parser::parse_visibility` and wrap the result in a `Nonterminal::NtVis` (also to be added).  The parser should be modified such that `Parser::parse_visibility` detects and unpacks such nonterminals, allowing them to be substituted without the need for reparse tricks.

The `vis` matcher's follow set should consist of *at least* the following tokens:

    <Ident> <Comma>
    const enum extern fn mod
    static struct trait type use

The `priv` reserved keyword should be excluded from the follow set, on the basis that it *might* be re-introduced as a visibility qualifier in the future.

## Test Case (Normative)

*Ideally*, the following source file should compile and run with this change.  It is possible the tests may need to be adjusted for practical considerations during implementation.

```rust
#![allow(dead_code, unused_imports)]

/**
Ensure that `:vis` matches can be captured in existing positions, and passed
through without the need for reparse tricks.
*/
macro_rules! vis_passthru {
    ($vis:vis const $name:ident: $ty:ty = $e:expr;) => { $vis const $name: $ty = $e; };
    ($vis:vis enum $name:ident {}) => { $vis struct $name {} };
    ($vis:vis extern "C" fn $name:ident() {}) => { $vis extern "C" fn $name() {} };
    ($vis:vis fn $name:ident() {}) => { $vis fn $name() {} };
    ($vis:vis mod $name:ident {}) => { $vis mod $name {} };
    ($vis:vis static $name:ident: $ty:ty = $e:expr;) => { $vis static $name: $ty = $e; };
    ($vis:vis struct $name:ident;) => { $vis struct $name; };
    ($vis:vis trait $name:ident {}) => { $vis trait $name {} };
    ($vis:vis type $name:ident = $ty:ty;) => { $vis type $name = $ty; };
    ($vis:vis use $path:ident as $name:ident;) => { $vis use self::$path as $name; };
}

mod with_pub {
    vis_passthru! { pub const A: i32 = 0; }
    vis_passthru! { pub enum B {} }
    vis_passthru! { pub extern "C" fn c() {} }
    vis_passthru! { pub mod d {} }
    vis_passthru! { pub static E: i32 = 0; }
    vis_passthru! { pub struct F; }
    vis_passthru! { pub trait G {} }
    vis_passthru! { pub type H = i32; }
    vis_passthru! { pub use A as I; }
}

mod without_pub {
    vis_passthru! { const A: i32 = 0; }
    vis_passthru! { enum B {} }
    vis_passthru! { extern "C" fn c() {} }
    vis_passthru! { mod d {} }
    vis_passthru! { static E: i32 = 0; }
    vis_passthru! { struct F; }
    vis_passthru! { trait G {} }
    vis_passthru! { type H = i32; }
    vis_passthru! { use A as I; }
}

mod with_pub_restricted {
    vis_passthru! { pub(crate) const A: i32 = 0; }
    vis_passthru! { pub(crate) enum B {} }
    vis_passthru! { pub(crate) extern "C" fn c() {} }
    vis_passthru! { pub(crate) mod d {} }
    vis_passthru! { pub(crate) static E: i32 = 0; }
    vis_passthru! { pub(crate) struct F; }
    vis_passthru! { pub(crate) trait G {} }
    vis_passthru! { pub(crate) type H = i32; }
    vis_passthru! { pub(crate) use A as I; }
}

/*
Ensure that the `:vis` matcher works in a more complex situation: parsing a
struct definition.
*/
macro_rules! vis_parse_struct {
    /*
    The rule duplication is currently unavoidable due to the leading attribute
    matching.
    */
    ($(#[$($attrs:tt)*])* pub($($vis:tt)*) struct $name:ident {$($body:tt)*}) => {
        vis_parse_struct! { @parse_fields $(#[$($attrs)*])*, pub($($vis)*), $name, $($body)* }
    };
    ($(#[$($attrs:tt)*])* pub struct $name:ident {$($body:tt)*}) => {
        vis_parse_struct! { @parse_fields $(#[$($attrs)*])*, pub, $name, $($body)* }
    };
    ($(#[$($attrs:tt)*])* struct $name:ident {$($body:tt)*}) => {
        vis_parse_struct! { @parse_fields $(#[$($attrs)*])*, , $name, $($body)* }
    };
    
    (@parse_fields $(#[$attrs:meta])*, $vis:vis, $name:ident, $($fvis:vis $fname:ident: $fty:ty),* $(,)*) => {
        $(#[$attrs])* $vis struct $name { $($fvis $fname: $fty,)* }
    };
}

mod test_struct {
    vis_parse_struct! { pub(crate) struct A { pub a: i32, b: i32, pub(crate) c: i32 } }
    vis_parse_struct! { pub struct B { a: i32, pub(crate) b: i32, pub c: i32 } }
    vis_parse_struct! { struct C { pub(crate) a: i32, pub b: i32, c: i32 } }
}

fn main() {}
```

# Drawbacks
[drawbacks]: #drawbacks

It's more code to maintain, and an extra bit of complication for the already fairly complicated macro system.  The compensation for this is that it makes the macros *themselves* less complicated.

# Alternatives
[alternatives]: #alternatives

- Do nothing and drive the people writing macros ever closer to complete mental breakdown.

- Dramatically expand `macro_rules!` such that it is expressive enough to represent something akin to `$vis:( $| pub $($($:tt)*)? )` (bind submatch with alternation and a zero-or-one group).

# Unresolved questions
[unresolved]: #unresolved-questions

- Should the matcher be called `vis` or something else?
