- Feature Name: (`lisp_impl`)
- Start Date: (2022-04-01)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC extends Greenspun's tenth rule to cover the Rust Standard Library.

# Motivation
[motivation]: #motivation

Greenspun's tenth rule, [in its original form,][1] states that it only applies to `C` or `Fortran` programs. However, there is an opportunity for the rule to be expanded, by including `Rust` under its umbrella. The expected outcome is that there will be a documented interface to use the interpreter (with bugs and slowness included).

[1]: https://philip.greenspun.com/research/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Contained in the `std::greenspun` module is a buggy [Lisp][2] interpreter that can be called programmatically from any Rust program that links the standard library.

[2]: https://en.wikipedia.org/wiki/Lisp_(programming_language)

## Examples

Evaluating a Lisp program embedded in a macro:
```rust
use std::greenspun::{li, evaluate, LispVal};

let a = 1.;

let program = li!(car (cons {a} 2.5));

let result = evaluate(program)?;

assert_eq!(result, LispVal::from(2.5));
```

Invalid Lisp may be rejected, complete with bad errors to show to end users:

```rust
use std::greenspun::{li, evaluate};

let program = li!(cons 1. 2. 3.);

assert_eq!(evaluate(program), String::from("too_many_PARAMS"));
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Lisp interpreter is run with the function `std::greenspun::evaluate`. It takes a `LispVal` representing the program, and returns as its result another `LispVal`. This initial implementation only allows `f64` numbers as values.

## API

```rust
// std::greenspun

macro_rules! li(..);

#[non_exhaustive]
enum LispVal {
    Pair(Rc<LispVal>, Rc<LispVal>),
    Num(f64),
}

fn evaluate(value: LispVal) -> Result<LispVal, String>;
```

# Drawbacks
[drawbacks]: #drawbacks

The date.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

A program wanting a Lisp interpreter already has many other options, since it can just link a C or Fortran program that already has its implementation of Lisp.

# Prior art
[prior-art]: #prior-art

None noticed

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What sort of API would be the most buggy way to expose the interpreter?

# Future possibilities
[future-possibilities]: #future-possibilities

The implementation of languages could be expanded to implementations of other languages, such as Fortran or C.
