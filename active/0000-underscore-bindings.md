- Start Date: 2014-10-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, bindings of the form

```rust
let _ = ...;
```

are treated differently than bindings of the form

```rust
let _x = ...;
```

where `_x` is a fresh unused variable. In particular, the destructor for `_`
will run immediately, whereas the destructor for `_x` will run at the end of
the scope.

# Motivation

Currently, this program prints `baab`:

```rust
struct A;

impl Drop for A {
    fn drop(&mut self) {
        print!("a");
    }
}

fn f() {
    let _a = A;
    print!("b");
}

fn g() {
    let _ = A;
    print!("b");
}

fn main() {
    f();
    g();
    print!("\n");
}
```

It would make more sense if the `_` binding in `g` were treated the same as the
`_a` binding in `f` and its destructor ran at the end of the scope. Then this
program would print `baba`.

When it comes to argument bindings, both cases behave identically. For example,
the following program prints `baba`: 

```rust
struct A;

impl Drop for A {
    fn drop(&mut self) {
        print!("a");
    }
}

fn f(_a: A) {
    print!("b");
}

fn g(_: A) {
    print!("b");
}

fn main() {
    f(A);
    g(A);
    print!("\n");
}
```

Argument and non-argument bindings should be consistent.

# Detailed design

The proposed change is to make a `_` binding behave identically in all cases to
a binding `_x` where `_x` is a fresh unused variable.

# Drawbacks

I can't think of any reasons why someone would want to rely on the current
behavior. It would be more clear to explicitly scope the binding to specify that
the destructor should run immediately.

# Alternatives

The only reasonable alternative would be to preserve the existing behavior.

# Unresolved questions

There appear to be no unresolved questions.
