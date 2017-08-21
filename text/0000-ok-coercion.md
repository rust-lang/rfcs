- Feature Name: `ok_coercion`
- Start Date: 2017-08-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Implicitly coerce `()` to `Ok(())`.

# Motivation
[motivation]: #motivation

Idiomatic Rust uses the `Result` type to signal fallible execution. When a function has nothing to return on success,
one needs to append an `Ok(())` since a return value is required.

```rust
impl fmt::Display for EscapeUnicode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in self.clone() {
            f.write_char(c)?;
        }
        Ok(())      // <-- yuck.
    }
}
```

This is considered [strange][reddit-3bbi1l] and [unattractive][urlo-1578], since if the function is infallible, you
don't need to insert the `Ok(())` at all.

This RFC tries to address the issue by implicitly coerce `()` to `Ok(())`, achieving the parity with functions returning
`()`.

[reddit-3bbi1l]: https://www.reddit.com/r/rust/comments/3bbi1l/returning_a_resulterror/
[urlo-1578]: https://users.rust-lang.org/t/error-handling-design-in-horrorshow/1578/2

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Examples

We add a coercion rule from `()` to `Result<(), _>`, thus allowing the following code:

<table><tr><th>After this RFC</th><th>Before this RFC</th></tr><tr><td>

```rust
fn foo_0() -> Result<(), E> {
    println!("Hello!")
}
```

</td><td>

```rust
fn foo_0() -> Result<(), E> {
    println!("Hello!");
    Ok(())
}
```

</td></tr><tr><td>

```rust
fn foo_1() -> Result<(), E> {
    bar_1()?;
}
```

</td><td>

```rust
fn foo_1() -> Result<(), E> {
    bar_1()?;
    Ok(())
}
```

</td></tr><tr><td>

```rust
fn foo_2() -> Result<(), E> {
    if test_2() {
        bar_2()?;
    }
}
```

</td><td>

```rust
fn foo_2() -> Result<(), E> {
    if test_2() {
        bar_2()?;
    }
    Ok(())
}
```

</td></tr><tr><td>

```rust
fn foo_3() -> Result<(), E> {
    for item in items()? {
        if item.has_bar() {
            return;
        }
    }
    Err(E::NotFound)
}
```

</td><td>

```rust
fn foo_3() -> Result<(), E> {
    for item in items()? {
        if item.has_bar() {
            return Ok(());
        }
    }
    Err(E::NotFound)
}
```

</td></tr><tr><td>

```rust
fn foo_4() -> Result<(), E> {
    let result = loop {
        if bar_4() {
            break Err(E::OhNo);
        }
        break;
    };
    println!("{:?}", result);
    result
}
```

</td><td>

```rust
fn foo_4() -> Result<(), E> {
    let result = loop {
        if bar_4() {
            break Err(E::OhNo);
        }
        break Ok(());
    };
    println!("{:?}", result);
    result
}
```

</td></tr></table>

Although pointless, we would also insert `Some(())` for function returning `Option<()>`. In fact, this coercion is
applied for all types implementing `Try`.

<table><tr><th>After this RFC</th><th>Before this RFC</th></tr><tr><td>

```rust
fn foo_5() -> Option<()> {
    println!("wat");
}
```

</td><td>

```rust
fn foo_5() -> Option<()> {
    println!("wat");
    Some(())
}
```

</td></tr></table>

Coercion is not performed between generic `T` and `Result<T, _>`.

```rust
fn foo_6() -> Result<bool, E> {
    true
    //~^ ERROR [E0308]: mismatched types
    //   Must use `Ok(true)` instead.
}
```

## Changes to the Book

> (Modify Chapter 12.3 "*Refactoring to Improve Modularity and Error Handling*", section "[*Returning Errors from the
> run Function*][book-12-03]". Changes are **<ins>bolded and underlined</ins>**)

With the remaining program logic separated into the run function rather than being in main, we can improve the error
handling like we did with `Config::new` in Listing 12-9. Instead of allowing the program to panic by calling expect, the
run function will return a `Result<T, E>` when something goes wrong. This will let us further consolidate the logic
around handling errors in a user-friendly way into main. Listing 12-12 shows the changes to the signature and body of
run:

```rust
use std::error::Error;

// ...snip...

fn run(config: Config) -> Result<(), Box<Error>> {
    let mut f = File::open(config.filename)?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)?;

    println!("With text:\n{}", contents);
}
```

We've made **<ins>two</ins>** big changes here. First, we're changing the return type of the run function to
`Result<(), Box<Error>>`. This function previously returned the unit type, `()`, and we keep that as the value returned
in the `Ok` case. **<ins>Using `Result<(), _>` like this is the idiomatic way to indicate that we're calling `run` for
its side effects only; it doesn't return a value we need.</ins>**

For our error type, we're using the *trait object* `Box<Error>` (and we've brought `std::error::Error` into scope with a
use statement at the top). We'll be covering trait objects in Chapter 17. For now, just know that `Box<Error>` means the
function will return a type that implements the `Error` trait, but we don't have to specify what particular type the
return value will be. This gives us flexibility to return error values that may be of different types in different error
cases.

The second change we're making is removing the calls to expect in favor of `?`, like we talked about in Chapter 9.
Rather than `panic!` on an error, this will return the error value from the current function for the caller to handle.

**<ins>This function should return</ins>** an `Ok` value in the success case. We've declared the `run` function's
success type as `()` in the signature, which means we **<ins>should</ins>** wrap the unit type value in the `Ok` value.
**<ins>We could explicitly return a value of `Ok(())`, but this syntax looks strange and does not add much value to the
code, thus the compiler allows us to leave it off when the result is exactly `Ok(())`.</ins>**

When you run this, it will compile, but with a warning:

```text
warning: unused result which must be used, #[warn(unused_must_use)] on by default
  --> src/main.rs:39:5
   |
39 |     run(config);
   |     ^^^^^^^^^^^^
```

Rust is telling us that our code ignores the `Result` value, which might be indicating that there was an error. We're
not checking to see if there was an error or not, though, and the compiler is reminding us that we probably meant to
have some error handling code here! Let's rectify that now.

[book-12-03]: https://doc.rust-lang.org/book/second-edition/ch12-03-improving-error-handling-and-modularity.html#handling-errors-returned-from-run-in-main

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Coercion

We amend [RFC 401] to add a coercion rule from `()` to `T` if `T` implements `Try<Ok = ()>`. The coerced value will be
created as `Try::from_ok(())`.

Since coercion site happens outside of the end of a block, this means the following statements will now become valid:

```rust
// let is a coercion site:
let _: Result<()> = ();

// array literal is a coercion site:
let _: Result<()> = [Ok(()), (), Err(e)];
let _: Result<()> = [(); 3];

// tuple is a coercion site:
let _: (Result<()>, Result<()>) = ((), ());

// box expression is a coercion site:
let _: Box<Result<()>> = box ();
let _: Box<Result<()>> = HEAP <- ();
```

While coercion is transitive, all transitive rules are irrelevant to `()` → `impl Try`:

| Rule | Why irrelevant |
|------|----------------|
| subtyping | `()` has no lifetime, `()` can only be the subtype of itself |
| `&mut T` → `&T` | `Try` is not implemented for mutable references |
| `*mut T` → `*const T` | `Try` is not implemented for mutable pointers |
| `&T` → `*const T` | `Try` is not implemented for shared references |
| `&mut T` → `*mut T` | `Try` is not implemented for mutable references |
| closure → `fn` | `Try` is not implemented for closures |
| CoerceUnsized | `CoerceUnsized` is not implemented for `Result`/`Option` |
| `()` → `impl Try` | `Try` is not implemented for `()` |

It should be explicitly mentioned that the `()` → `impl Try` rule does not participate in "receiver coercion" (similarly
the closure → `fn` also does not participate), to avoid truly nonsense expressions like:

```rust
fn wut() -> u32 {
    ().unwrap_err()
    // ^ Do not coerce `()` into `Result<(), u32>`.
}
```

[RFC 401]: https://github.com/rust-lang/rfcs/blob/master/text/0401-coercions.md
[rfc-cmt-322834130]: https://github.com/rust-lang/rfcs/pull/2107#issuecomment-322834130

## `catch` expression

The `catch` block does not need to participate in the implicit `Ok` coercion. In fact, we should clarify the behavior of
`catch` expression that, according to [RFC 243][rfc-243-catch], if the `catch` expression results in a `Result<T, E>`,
the last statement of the block must return `T`. That is, the value of `catch { Ok("wut") }` is `Ok(Ok("wut"))`.
Following RFC 243 makes it naturally support:

```rust
let a: Result<()> = catch {
    foo()?;
    bar()?;
    // no need to coerce `()` into `Ok(())` here.
};
```

The equivalence relation should now be stated as:

<table><tr><td>

```rust
fn outside() -> Result<T, E> {
    inside()
}
```

</td><td>is equivalent to</td><td>

```rust
fn outside() -> Result<T, E> {
    catch {
        inside()? // <- add `?` at the end.
    }
}
```

</td></tr></table>

[rfc-243-catch]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#catch-expressions

# Drawbacks
[drawbacks]: #drawbacks

## Coercion may happen in unexpected places

For instance, the following will unexpectedly always return `Ok(())`:

```rust
fn typo() -> Result<(), E> {
    if cond {
        Ok(())
    } else {
        Err(E::OhNo);  // <- typo
    }
}
```

Coercion from `()` also allow nonsense expressions like

```rust
fn x(a: Result<(), E>) -> R { ... }

// compiles!
x(());
```

## Can only handle `Ok(())`

This RFC only treats `Ok(())` as special, that means if the function is going to return a generic `Result<T, E>`, an
explicit `Ok` is still needed.

```rust
fn parse_u8(a: &str) -> Result<u8, MyError> {
    Ok(a.parse()?)
    // ^ the Ok(...) is still needed.
}
```

This may confuse new users (and provoke language purists) that sometimes the `Ok` is needed, and sometimes not.

# Rationale and Alternatives
[alternatives]: #alternatives

## Rationale and alternatives

The RFC to solve the `Ok(())` issue was first proposed as [RFC 2107] "*Ok wrapping: Improved support for writing code
from an error handling mindset*". The RFC received a lot of downvotes, as some people think it changes the syntax too
much, while some thinks it does too little. This RFC is proposed as an alternative solution to `Ok(())`.

This RFC is written to respect the following restrictions:

1. **`Ok(())` is entirely eliminated**. We should agree requiring user to add `Ok(())` is just noise that serves no
    purpose other than satisfying the type checker. Any solution that requiring user to type something an extra
    keystroke is inferior to eliminating an trace of `Ok(())` completely.

2. **Keep the function signature unchanged**. Implicit `Ok(())` insertion is an implementation detail, requiring a
    change to function signature to toggle an implementation detail is overstepping.

3. **Compatible with Rust 2015 (1.0)**. This RFC does introduce any new syntax, thus does not require introducing a new
    checkpoint/epoch/major-version/delivery/whatever, allowing this to be deployed as soon as possible.

[RFC 2107]: https://github.com/rust-lang/rfcs/pull/2107

Not everyone may agree to the above rationales. The following lists some more alternatives raised during discussion of
RFC 2107, from the most conservative to the most aggresive.

### Do nothing

¯\\\_(ツ)\_/¯

### Alias `Ok(())`

The worst impression of `Ok(())` is the double parenthesis. Some suggests to change it to:

* [`Ok!()`](https://github.com/rust-lang/rfcs/pull/2107#issuecomment-323530196)
* `Ok()`
* [`Ok`](https://github.com/rust-lang/rfcs/pull/2107#issuecomment-323521532)
* [👌](https://internals.rust-lang.org/t/pre-rfc-throwing-functions/5419/14)

The first and fourth suggestions are trivially implementable. The second one requires the type-checker (which emits
E0061) to "upgrade" `X()` to `X(())` when applicable. The third is plain impossible since `Ok` itself is a valid value
of type `fn(T) -> Result<T, E>`.

No matter which choice, the solution is undesirable as the user still needs to add an extra expression at the end of the
function manually.

```rust
fn foo_1a() -> Result<(), E> {
    bar_1a()?;
    Ok  // <- no, not ok.
}
```

### Automatic `Ok` insertion via AST pass

Create an AST or HIR pass which automatically insert `Ok(())` at the end of a block if the block is not returning an
expression. The advantage is that the transformation does not touch the type-checker, and thus can be implemented much
more easily than this RFC. However, the rules of the syntactical pass is much more complex to explain, while teaching
implicit coercion is basically just a one-line description.

### Annotated `Ok` wrapping i.e. RFC 2107

When a function acquires a special annotation, the result of the function will be automatically wrapped in `Ok`. This is
what proposed in RFC 2107:

```rust
fn rfc2107_example()? -> Result<T, E> {
    //              ^ add this to enable Ok wrapping
    rfc2107_inner()?;
    return_value
    // ^ value of the block must be `T`, not `Result<T, E>`.
}
```

Some other syntactic possibilities with similar effects are:

* `fn foo()? -> Result<T, E> {`
* [`fn foo() -> Result<T, E> catches {`](https://github.com/rust-lang/rfcs/pull/2107#issuecomment-322290289)
* [`fn foo() -> Result<T, E> catch {`](https://github.com/rust-lang/rfcs/pull/2107#issuecomment-322849867)
* [`fn foo() -> Result<T, E>? {`](https://github.com/rust-lang/rfcs/pull/2107#issuecomment-322391046)
* `#[catches] fn foo() -> Result<T, E> {`

The main issue of this is a change of syntax is necessary to opt-in for `Ok` wrapping. There is also a minor annoyance,
that errors must be thrown using the syntax `Err(E::Stuff)?;` instead of `return Err(E::Stuff);`.

Additionally, the annotation controls an implementation detail of the function, but it will affect the source code
surrounding the function signature. While the `mut` modifier on argument pattern already breaks this rule, having one
misfeature doesn't mean we should introduce more to the language.

### Implicit coercion for `()` (This RFC)

😇

### Implicit coercion for generic `T`

Instead of coercing `()` to `impl Try<Ok=()>`, we could instead add the coercion rule from any `T` to `impl Try<Ok=T>`,
thus eliminating all explicit `Ok` at the return value.

Unlike the current RFC, this alternative rule will have much more unfavorable side-effects since the transitive rule is
now in full power. This would make `T` be coercible to `Result<Option<T>, _>`, or makes it hard for human to decide what
should `Err(4): Result<Result<_, i32>, u32>` become.

### Throwing function

As RFC 2107 already proposed to change the syntax, one may wonder why don't we just redesign it entirely for better
ergonomics. This became the central theme of RFC 2107's discussion on how to adapt this syntax:

<table><tr><td>

```rust
fn throwing_function() -> T throws E {
    throwing_function_inner()?;
    if cond() {
        throw E::Invalid;
    }
    value
}
fn throwing_void_function() throws E {
    xxx()?;
    yyy();
}
```

</td><td>is equivalent to</td><td>

```rust
fn throwing_function() -> Result<T, E> {
    throwing_function_inner()?;
    if cond() {
        return Err(E::Invalid);
    }
    Ok(value)
}
fn throwing_void_function() -> Result<(), E> {
    xxx()?;
    yyy();
    Ok(())
}
```

</td></tr></table>

The syntax was first mentioned by [RFC 243][rfc-243-throws] as a future possibility, and again proposed as a
[Pre-RFC][pre-rfc-5419] recently. This also has precedence in [Swift's throwing functions][swift-throws].

The design space around this is so large that it should better be proposed as a separate RFC to make writing fallible
functions in Rust 2018 more <!-- <del>like Java</del> --> readable, with solving the `Ok(())` problem being a nice
side-effect.

[rfc-243-throws]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#throw-and-throws
[pre-rfc-5419]: https://internals.rust-lang.org/t/pre-rfc-throwing-functions/5419
[swift-throws]: https://developer.apple.com/library/content/documentation/Swift/Conceptual/Swift_Programming_Language/ErrorHandling.html#//apple_ref/doc/uid/TP40014097-CH42-ID510

## Behavior of `catch` expression

This RFC proposes to change the implementation of `catch` to match the original definition in RFC 243:

<table><tr><th>Current rustc behavior</th><th>RFC 243 behavior</th></tr><tr><td>

```rust
let a: Option<i32> = catch {
    Some(4)
};
let b: Result<&str, &[u8]> = catch {
    if foo()? {
        Err(&b"bad"[..])
    } else {
        Ok("good")
    }
};
```

</td><td>

```rust
let a: Option<i32> = catch {
    4
};
let b: Result<&str, &[u8]> = catch {
    if foo()? {
        Err(&b"bad"[..])?
    } else {
        "good"
    }
};
```

</td></tr></table>

RFC 2107, however, proposes to *keep* behavior of `catch` match the current implementation, and introduce `catch?` to
match the RFC 243 behavior. This makes sense only for RFC 2107 since this corresponds to the function annotation. But
introducing `catch?` is pointless outside of RFC 2107.

Besides, the existing rustc behavior can be reproduced using a closure:

```rust
let a: Option<i32> = (|| {
    Some(4)
})();
let b: Result<&str, &[u8]> = (|| {
    if foo()? {
        Err(&b"bad"[..])
    } else {
        Ok("good")
    }
})();
```

## Tweaking this RFC

* Stop relying on `Try`, and introduce a special trait as `()` → `EmptyTailExpr` coercion, and implement
    `EmptyTailExpr` for `Result<(), _>`.

* Restrict to `Result` only, don't care about `Option`/`Poll`.

* Restrict coercion site for this particular rule to the end of a block and `return`.

# Unresolved questions
[unresolved]: #unresolved-questions

None
