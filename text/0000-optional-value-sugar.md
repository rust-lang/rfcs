- Feature Name: optional-value-sugar
- Start Date: 9-10-2018
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Sugar for optional value to make using them more clear and easier to use when writing functions. `t: T?` is an optional value of type `T`.

# Motivation
[motivation]: #motivation

This will formalize a common pattern, and make it more expressive in a backwards compatible way. It will also allow for easy pattern for default values.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

```Rust
fn safe_log(value: f32, base: f32?) -> f32? {
    if value == 0.0 {
        None
    } else {
        let base = base.unwrap_or(10.0);
        if base == 1.0 || base == 0.0 {
            None
        } else {
            f32::log(value, base)
        }
    }
}

fn main() {
    assert_eq!(safe_log(1.0, None), Some(0.0));
    assert_eq!(safe_log(1.0, 2.0), Some(0.0));
    assert_eq!(safe_log(3.0, 0.0), None);
    assert_eq!(safe_log(2.0, 1.0), None);
    assert_eq!(safe_log(0.0, 2.0), None);
    assert_eq!(safe_log(0.0, 1.0), None);
}
```

This will desugar to

```Rust
fn safe_log(
    value: f32,
    base: impl Into<Option<f32>> // This allows you to write the code in main, without using Some(...) at the call site
) -> Option<f32> // This is a simple transform
{
    // This is added, because it is the only thing you can do with base.
    // Other than, storing it, and if you are storing it, you should be more explicit about it
    let base = base.into();
    if value == 0.0 {
        None
    } else {
        let base = base.unwrap_or(10.0);
        if base == 1.0 || base == 0.0 {
            None
        } else {
            f32::log(value, base).into() // This into is added, so you can convert into an Option, without writing Some(...), this will also allow other conversions, and that is fine, we could also change to auto wrap the value in a Some
        }
    }
}

fn main() {
    // Nothing changes here
    assert_eq!(safe_log(1.0, None), Some(0.0));
    assert_eq!(safe_log(1.0, 2.0), Some(0.0));
    assert_eq!(safe_log(3.0, 0.0), None);
    assert_eq!(safe_log(2.0, 1.0), None);
    assert_eq!(safe_log(0.0, 2.0), None);
    assert_eq!(safe_log(0.0, 1.0), None);
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are three places where this sugar will need to be considered: function arguments, function return type, and function call site. This sugar will also allow for ergonomic defaults.

This RFC takes advantage of these two impl.

```Rust
/// Impl 1
impl From<T> for T {
    fn from(value: T) -> Self {
        T
    }
}

/// Impl 2
impl From<T> for Option<T> {
    fn from(value: T) -> Self {
        Some(value)
    }
}
```
To create a backwards compatible ergonomic optional value sugar.

## Function Arguments

```Rust
fn maybe_print(x: i32?) {
    match x {
        Some(x) => println!("{}", x),
        None => ()
    }
}

// desugars to

fn maybe_print(x: impl Into<Option<i32>>) {
    let x = x.into();
    match x {
        Some(x) => println!("{}", x),
        None => ()
    }
}
```

## Function Return Type (Optional)

```Rust
fn maybe_return(do_return: bool) -> i32? {
    if do_return {
        0
    } else {
        None
    }
}

// desugars to

// 1
fn maybe_return(do_return: bool) -> i32? {
    if do_return {
        0.into()
    } else {
        None.into()
    }
}

// or

// 2
fn maybe_return(do_return: bool) -> i32? {
    if do_return {
        Some(0)
    } else {
        None
    }
}
```

## Function call site

No necessary or breaking changes, hooray! Because of impl 1 and impl 2, we can change to sugar without fear.

```Rust
fn main() {
    maybe_print(12); // because of Impl 2 (new style)
    maybe_print(Some(42)); // because of Impl 1 (old style)
    maybe_print(None); // because of Impl 1 (old + new style)
    
    assert_eq!(maybe_return(true), Some(0)); // No changes here
    assert_eq!(maybe_return(false), None); // No changes here
}
```

## Ergonomic Defaults

```Rust
struct Foo(i32);

fn build(foo: i32?) -> Foo {
    // You can use this for defaults, and this behaviour can be documented
    let foo = foo.unwrap_or(10);
    Foo(foo)
}

fn main() {
    let a = build(None); // use default
    let a = build(20); // use my value
}
```

# Drawbacks
[drawbacks]: #drawbacks

 - Adds complexity to the language that is not needed, but will be very nice, and it will make it easier to read code.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

 - Not doing this

## More sugar! (for defaults)

```Rust
fn build(foo: i32? = 10) -> Foo {
    Foo(foo)
}

// desugars to

fn build(foo: impl Into<Option<i32>>) -> Foo {
    let foo = foo.into().unwrap_or(10);
    Foo(foo)
}
```

as long as the default is a `const`

## 

# Prior art
[prior-art]: #prior-art

Languages like C#, Kotlin, and Swift have this sugar.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Do we want to have sugar for optional return values?