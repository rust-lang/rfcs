- Feature Name: function_structs
- Start Date: 2018-01-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This will allow functions to be defined as structs in order to allow optional/default arguments and keyword arguments.

# Motivation
[motivation]: #motivation

Optional and keyword arguments are an often requested feature, but didn't make it into the language yet.
This is a simple way to support keyword arguments for functions, without having to change much of the language.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When you define a function, a matching struct will be defined automatically when needed.
It will implement a new method `apply(self)` for it, which then calls the function with the fields of the struct as arguments and returns its value.

```rust
// you write this:
fn     test(    a: A,     b: B) -> C { C::new() }
// this gets implemented automatically:
struct test{pub a: A, pub b: B}
impl test {
    fn apply(self) -> C {
        test(self.a, self.b)
    }
}
```

It already is possible to implement a function this way. The example should compile.

Instead of calling the function directly it can now be called in different ways:
```rust
// normal way:
test(A::new(), B::new())
// new way:
test{a: A::new(), b: B::new()}.apply()
```

This doesn't seem useful yet, but it is also possible to implement other functions and traits for these function objects.
For example one could implement the trait `Default` for the object and set default function arguments for a function:

```rust
fn test(a: A, b: B, c: C, d: D) {/*...*/}

impl Default for test {
    fn default() -> Self {
        Self {
            /*...*/
        }
    }
}

//now call it:

test{c: C::new(), ..Default::default()}.apply()
```

It's also possible to define some required arguments and some optional arguments for the function struct by using functions, which return a function struct.

You can also use derive with functions. These will apply to implicitely created structs.
For example by deriving `Clone` and optionally `Copy` it's easier to reuse the argument lists for functions without rewriting the same list for every function:

```rust
#[derive(Copy, Clone)]
fn test(a: i32, b: i32, c: i32) -> i32

let mut test = test{a: 1, b: 2, c: 3};
println!("test({}, {}, {}) = {}", test.a, test.b, test.c, test.apply());
test.b = 4;
println!("test({}, {}, {}) = {}", test.a, test.b, test.c, test.apply());
```

Functions with `self` as an argument are not be convertable to function structs, else `apply(self)` will also create a new function struct. Also it would be required to accept `self` as a keyword for these structs.

If a struct with the same name as the function is defined in the same module, it will overwrite the implicite definition, so it won't break anything, and allows to define own implementations of similar things (for example using `apply(&mut self)` instead).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

It could be implemented using (procedural) macros, which get implicitely called, which automatically creates the correct struct definitions.
It's needed to add the implicite lifetimes from the function to the structs.
Instead it may be preferable to only implement function structs for structs, that are used as function structs. The implementation of `apply(self)` may then also be optimized.

# Drawbacks
[drawbacks]: #drawbacks

Maybe it won't be used that much, but slows down the compiliation process.

# Rationale and alternatives
[alternatives]: #alternatives

It may be possible to use a trait `Apply`, which contains `apply(self)` instead of implementing it for the type itself, but it seems to be not that useful.

It could be implemented using (procedural) macros, but then only function structs can be used, when the crate defines them itself. Then it's also difficult to optimize the implementation of `apply(self)`.

# Unresolved questions
[unresolved]: #unresolved-questions

Should methods containing `self` also be supported? (would not be possible by just using macros)

Should function struct fields be public by default, or do they have to be declared public inside the function declaration?

Should it be forbidden, that functions and structs with the same name exists in the same module, because noone uses them anyway, and there is a warning for lowercase chars?
