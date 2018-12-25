- Feature Name: local_loop_bindings
- Start Date: 2018-12-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

To simplify complicated loop constructs and avoid mutable state,
allow an extended syntax for `loop` to accept local variables that may change once per iteration.


# Motivation
[motivation]: #motivation

The new syntax is inspired by `loop` in the upcoming release of the [scopes programming language](scopes.rocks).
The chief motivation is to enable using different values for each iteration without the need of mutable bindings defined outside of the loop.

The variables will be defined after the loop keyword, so they will only be accessible in the scope of the loop, not afterwards. They will not be mutable by default, so it can be ensured, that the variables only change once per iteration.

Especially since loops can return values, it's not necessary at all to mutate state inside a loop in some cases.

This is a more functional programming style, which may also allow more optimizations like storing the loop arguments in registers instead of allocating storage for mutable variables.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The extended syntax for `loop` will just work like a `while let`. "while let" will be replaced by "loop". Unlike `while let`, the pattern is not allowed to be refutable, so enum variants are not allowed on the lefthand side of "=".
The introduced bindings will be accessible in this loop.

The new syntax will look like this:

```rust
loop binding = value {
    /* body */
}
```

The return value of the loop body will implicitely be passed to the next iteration of the loop, so it needs to be the same type as the initial value.

An example of a simple iteration, which iterates the loop ten times and prints the iteration number would look like this:

```rust
loop i = 1 {
    if i <= 10 {
        println!("iteration {}", i);
        i + 1
    } else {
        break;
    }
}
```

`continue` will accept an argument in this loop, which will be passed to the next iteration. Using continue, this could look like this:

```rust
loop i = 1 {
    if i <= 10 {
        println!("iteration {}", i);
        continue i + 1;
    }
    break;
}
```

Since the end of the loop is never reached, the return value is not required to be the type of the binding, here.

A loop without bindings (`loop { /* body */ }`) will be the same as this:

```rust
loop () = () {
    /* body */
}
```

This will not be a breaking change, since it's not allowed to have values other than `()` from a loop.

A simple example from the book looks like this:

```rust
let mut x = 5;
let mut done = false;

while !done {
    x += x - 3;

    println!("{}", x);

    if x % 5 == 0 {
        done = true;
    }
}
```

Using the new syntax, this could be rewritten as this:

```rust
loop (mut x, done) = (5, false) {
    if done {
        break;
    }
    x += x - 3;

    println!("{}", x);

    if x % 5 == 0 {
        (x, true)
    } else {
        (x, false)
    }
}
```

This is, how you would define factorial using a loop now:

```rust
fn factorial(x: i32) -> i32 {
    loop (result, count) = (1, x) {
        if count == 1 {
            break result;
        }
        (result * count, count - 1)
    }
}
```

With explicit `continue`, it can look like this:

```rust
fn factorial(x: i32) -> i32 {
    loop (result, count) = (1, x) {
        if count == 1 {
            break result;
        } else {
            continue (result * count, count - 1);
        }
    }
}
```

Using `break` here allows copying code without having to modify it, when not using a specific function.

Labels will also work. When using `continue` with a label, the arguments to continue must match the loop binding signature connected to the label, in case the label is connected with a loop.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The syntax extension should not cause any issues with backwards compatability.

It's just an extended syntax for `loop` in a place, where currently nothing is allowed yet.

The expansion of the new syntax will be shown for an example.

New syntax:

```rust
loop (a, mut b) = (x, y) {
    /* body */
}
```

Current syntax:

```rust
{ // ensure global bindings to be inaccessible after the loop
    let mut binding = (x, y);
    loop {
        let (a, mut b) = binding;
        binding = {
            /* body */
        }
    }
}
```

This expansion should cover the common case.

A `continue value` in the body would expand to `binding = value; continue;

Internally there may be more efficient ways to implement this.


# Drawbacks
[drawbacks]: #drawbacks

This adds more options to the language, which also makes the language more complicated, but it should be pretty intuitive, how it works.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

It would be possible to extend `while let` instead, so it supports both refutable and irrefutable value and add additionally add support for `continue`, but in one case the expression generating the value is called for each iteration and in the other case only in the beginning, so this is probably not an option.

To avoid confusion, it would be possible to require a `continue` branch to repeat. Any branch reaching the end without `continue` would fail.

It would also be possible to just have labeled blocks with bindings, similar to "named let", as known from Scheme. In this case, reaching the end of the block will just leave the loop and go on afterwards.
This could be a more general version, which is not connected to loops, but can be used for everything, which can have labels.


# Prior art
[prior-art]: #prior-art

Without the feature of loops being able to return values, this feature is less useful.

Labeled blocks, which are currently unstable, may also be useful for some alternative to this.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are some other design decisions, mentioned as alternatives, which could be taken into account instead.
But I'm pretty sure, the proposal itself is more useful and straightforward than the alternatives.
There are no unresolved questions yet.

# Future possibilities
[future-possibilities]: #future-possibilities

If named blocks are stabilized, they could additionally allow local bindings, like a "named let".

