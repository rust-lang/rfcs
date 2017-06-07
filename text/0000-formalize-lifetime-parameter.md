- Start Date: 2014-12-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

(1) Formalize the term *lifetime parameter* when describing `'a`, `'b`..., and (2) call it lifetime parameter `'a`,  lifetime parameter `'b` etc.

(Alternative: use the term *lifetime bound*.)

# Motivation

### 1. Formalize the term “lifetime parameter” when describing `'a`

The term to describe `'a` is inconsistent even in the Rust compiler source code itself. Doing a `grep` gives us some rough idea:

    $ git clone https://github.com/rust-lang/rust.git
    $ cd rust/src
    $ grep -ir -e "lifetime '" -e "lifetime \`'" . | wc -l
          30
    $ grep -ir "lifetime parameter" . | wc -l
         105
    $ grep -ir "lifetime bound" . | wc -l
          30
    $ grep -ir "lifetime specifier" . | wc -l
           9
    $ grep -ir "named lifetime" . | wc -l
           9

There are at least five different usages: describing it as a lifetime, lifetime parameter, lifetime bound, lifetime specifier, or named lifetime. Instead of letting so many variations float around, we should formalize a term. Since *lifetime parameter* gets more frequently used, we may choose it.

### 2. Call it lifetime parameter `'a` instead of lifetime `'a`

We should use the term we formalized in the previous section to call `'a`. Calling it lifetime `'a` is incorrect because it is not the lifetime of any resource, pointer or binding. For example:

```rust
struct Foo { f: Box<int> }
struct Link<'a> { link: &'a Foo }

fn store_foo<'a>(x: &mut Link<'a>, y: &mut Link<'a>, a: &'a Foo, b: &'a Foo) {
    x.link = if a.f > b.f { a } else { b };
    y.link = b;
}
```

In function `store_foo`, `'a` dictates that both `*a` and `*b` outlive `*x` and `*y`. Experienced Rust programmers know `'a` is not anyone’s lifetime, but a  minimal lifetime requirement or bound. Calling it lifetime is just incorrect.

Although calling it lifetime parameter `'a` does not tell you anything, it is not wrong.

Alternatively, we may call it lifetime bound `'a` which perhaps makes more sense, and formalize the term *lifetime bound* across the codebase instead.

# Detailed design

1. Unify the term. Whatever is chosen, use it consistently in the Rust compiler code.
1. Apply the change to the compiler messages, API doc and official guides.

# Drawbacks

It reads longer: lifetime parameter `'a`.

# Alternatives

- Use the term *lifetime bound* instead of *lifetime parameter*.
- Unify the term for the first part but leave the second part as is, i.e. keep calling it lifetime `'a`.

# Unresolved questions

No.
