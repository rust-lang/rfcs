- Feature Name: Alternatives in patterns
- Start Date: 2016-02-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary
Extend the pattern syntax for alternatives in `match` statement, allow alternatives for pattern matching in `let` and `if let` statements.

# Motivation
[motivation]: #motivation

Rust allows alternatives ( `|` ) in pattern syntax for `match`, but only for 'top-level' of pattern.
This aims to reduce verbosity in certain examples and increase expressiveness.

Also, this RFC proposes to allow alternatives in `let` or `if let` statements.

# Detailed design
[design]: #detailed-design

## Expand usage of alternatives to 'deeper levels'
Imagine a type:
```rust
struct NewType (Result<String,String>, u8);
```

Exhaustive `match` statement for this type would look like this:
```rust
match new_type {
    NewType(Ok(e), num) | NewType(Err(e), num) => println!("ok with {}: {}", num, e)
}
```

This RFC proposes a following replacement:
```rust
match new_type {
    NewType(Ok(e) | Err(e), num) => println!("ok with {}: {}", num, e)
}
```


Little bit more complicated example:
```rust
enum Test { First, Second }
//Current Rust:
match (test1, test2) {
    (First, First) | (First, Second) | (Second, First) | (Second, Second) => println!("matches")
}

//This RFC proposes:
match (test1, test2) {
    (First| Second, First | Second) => println!("matches")
}
```

## Allow alternatives in `let` statements
Currently expressions like `Ok(e) | Err(e)` are not allowed in `let` or `if let` statements, which brings inconsistence to pattern matching.  
This RFC proposes following to be allowed:
```rust
if let (First | Second) = three_variants_enum {}
let (Ok(e) | Err(e)) = result;
let closure = |(Ok(e) | Err(e))| println!(e); // works similar to the statement above
```

### Parentheses around patterns in `let` statements
Multiple alternatives should be enclosed in parentheses and represent a single pattern,
while single alternative should not be enclosed with parens to be backwards compatible.
```rust
enum Three{ A(i32), B(i32), C(i32) }
if let A(i) = three {}
if let (A(i) | B(i)) = three {}
```
Parens should be introduced due to:
- Pattern matching in closure arguments:

```rust
let closure = | Ok(i) | Err(i) | i; // Is it possible to find pattern's end and actual closure' start?

let closure = |(Ok(i) | Err(i))| i; // As proposed by this RFC
```
- Follow rules for macros (as discussed in [1384#comment](https://github.com/rust-lang/rfcs/pull/1384#issuecomment-164275799))

### Irrefutable patterns
Patterns in `let` statements must be irrefutable - meaning they must cover every possible variant:
```rust
enum Three{ First(u8), Second(u8), Third(u8) }
//...
let (First(u) | Second(u)) = three; //Not allowed!
```

Patterns in `if let` statements should be disallowed to be irrefutable, it means they are not allowed to cover every possible variant:
```rust
if let (Ok(e) | Err(e)) = result {
  //Not allowed!
} else {}
```
If pattern is irrefutable, then an `else`-branch will never be executed, and `if` will be redundant.

# Drawbacks
[drawbacks]: #drawbacks

These features, probably, are not easy to implement.

# Alternatives
[alternatives]: #alternatives

- **This is a subset of [#99](https://github.com/rust-lang/rfcs/pull/99).** The original RFC was postponed and as suggested by [#1456](https://github.com/rust-lang/rfcs/issues/1456#issuecomment-173943563) a new RFC was created with a link to postponed one.
- **Implement the proposal only for `match`.** This has a downside of further increased inconsistence.
- **Allow irrefutable patterns in `if let` statements.** This way, `else`-branch will not always execute. If so, a warning about unreachable code should be emitted.

# Unresolved questions
[unresolved]: #unresolved-questions

- The possibility of treating single variant w/o parens as a pattern (as it is treated today) simultaneously with treating multiple variants with parens as a pattern.
- The requirement of parens around multiple alternatives in *deeper levels* of pattern matching, i.e is this legal:
```rust
match new_type {
    NewType(Ok(e) | Err(e), num) => println!("ok with {}: {}", num, e)
}
```
