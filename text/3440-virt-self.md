- Feature Name: `virt_self`
- Start Date: `2023-05-30`
- RFC PR: [rust-lang/rfcs#3440](https://github.com/rust-lang/rfcs/pull/3440)
- Rust Issue: [rust-lang/rust#3440](https://github.com/rust-lang/rust/issues/3440)

# Summary
[summary]: #summary

Enable virtual dispatch of trait methods.

# Motivation
[motivation]: #motivation

Coming to Rust from an OOP language such as C++ one is told to favor composition over inheritance. In general, I think that's a great thing. However, lack of language features such as delegation and what this RFC proposes can make certain patterns more difficult to express than they should be.

Consider the following situation. Say we have a trait `System` that models a given system.

```rust
pub trait System {
    fn op1(&self) -> SystemOp1;
    fn op2(&self) -> SystemOp2;
    // etc.
}
```

Assume that we now have a particular impl. `SystemA` which implements `op1` through a reference to `op2`. For instance,

```rust
// asume the following
type SystemOp1 = i64;
type SystemOp2 = i64;

pub struct SystemA {

}

impl System for SystemA {
    fn op1(&self) -> SystemOp1 {
        self.op2() * 5
    }
    fn op2(&self) -> SystemOp2 {
        3
    }
}
```

Assume we now want to have a general purpose wrapper that allows us to somehow map the result of `op2`, say. For instance,

```rust
pub struct DoubleOp2<S: System> {
    sys: S,
}

impl<S: System> System for DoubleOp2<S> {
    fn op1(&self) -> SystemOp1 {
        self.sys.op1()
    }
    fn op2(&self) -> SystemOp2 {
        self.sys.op2() * 2
    }
}
```

Clearly, this has the intended effect of changing `op2`. However, it also has the unintended effect of keeping `DoubleOp2<SystemA>::op1()` out of sync with `DoubleOp2<SystemA>::op2`. We got static dispatch when in this context virtual dispatch made more sense.

Of course, in Rust, we usually associate dynamic dispatch with the `dyn` keyword. However, `dyn System` only gives a vtable for a particular impl without virtualizing any subsequent calls. In other words, calling `op1` through `DoubleOp2<SystemA> as dyn System` will still call `SystemA::op2`.

Of course, this behaviour may be what is desired. But sometimes, virtualizing at depth is more appropriate. Certainly when adopting OOP patterns.

When I first stumbled upon this, I wondered if I could simply take `self` as `&dyn System` (or `Arc` or `Box` equivalent, etc.). Nope.

The code I was working on required me to keep the trait `System` as one unit so I did not investigate possible ways to split it up.

What I came up is passing a &dyn System virtualized self so now System looked like this

```rust
pub trait System {
    fn op1(&self, vself: &dyn System) -> SystemOp1;
    fn op2(&self, vself: &dyn System) -> SystemOp2;
}
impl System for SystemA {
    fn op1(&self, vself: &dyn System) -> SystemOp1 {
        vself.op2() * 5
    }
    fn op2(&self, vself: &dyn System) -> SystemOp2 {
        3
    }
}
```

And then calling `op2` with essentially another copy of `self`:
```rust
let system = DoubleOp2<SystemA> {};
system.op2(&system2); // works!
```

Of course this design is a bit clunky, but is very powerful. It allows complete control over what is virtualized and what isn't at any point in the call chain for every implementation.

As I have been thinking about this more and more, I wondered what Rust with this in the language would look like. This RFC proposes a sample syntax (bikeshedding welcome, but please focus on the concepts).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Essentially, we now allow `&virt self` as a shorthand for the above.

```rust
pub trait System {
    fn op1(&virt self) -> SystemOp1;
    fn op2(&virt self) -> SystemOp2;
}
```

The effect is that rust will manage two pointers - one concretely typed and one dynamic. The The concretely typed (i.e. `self`) follows the existing Rust rules. The dynamic one is either `self` as `dyn System` again or an existing dynamic `self` (this is like choosing `self` or `vself` in the previous section).

Syntax could be e.g.
```rust
impl System for SystemA {
    fn op1(&virt self) -> SystemOp1 {
        // alternative 1
        virt(self).op2() * 5
        // alternative 2, I like this one most because it reminds me of C
        self->op2() * 5
        // something else...
    }
    fn op2(&virt self) -> SystemOp2 {
        3
    }
}
```

Working with the second syntax, the difference between `self.op2()` and `self->op2()` would be static vs dynamic dispatch. In other words, it would allow us to call `DoubleOp2<SystemA>::op2()` from within `SystemA::op1()` when the call to it is made from `DoubleOp2<SystemA>::op1()`.

If the called method is also declared virt, using `self->op2()` will retain the `vself` the same as originally passed to `op1`. Otherwise, it will replace it by `self as &dyn System`.

Outside of traits, `a->b()` wouldn't compile. 

```[error-xxx] trait virtual methods can only be called virtually from within the trait```

Instead, only the traditional syntax of `a.b()` will be allowed and this would simply use a `vself` of `a as &dyn Trait`.

I believe this feature will make Rust more user-friendly to people more inclined to think in OOP terms or who (like me) simply found themselves writing code in a domain that is very amenable to an OOP approach.

The syntax changes are relatively minimal and there is no extra cost for code that does not use this feature.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementing the above for immutable references is easy. However, mut references are inherently unsafe as we will be aliasing self initially.

Solution is to either outright ban the above  for `&mut self` or only allow it in unsafe contexts (e.g. treat it as `mut ptr`).

# Drawbacks
[drawbacks]: #drawbacks

I fail to see any major drawbacks besides the override of `->`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?

It is intuitive and doesn't clutter the signatures unnecessarily.

- What other designs have been considered and what is the rationale for not choosing them?

Explicit argument passing considered above.

- What is the impact of not doing this?

The impact for experienced Rust users is probably low as they could figure their way to a solution. However, the impact for new Rust users coming from an OOP language may be great.

- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

Admittedly, a lot of the above can be done with a macro but it would be awkward to use on the initial call site. Also, the generated code would necessarily have to expose the `vself` parameter and the IDE experience may not be that great. Also, a macro would find it hard to differentiate between mut and immutable references and will lead to subpar error messages.

# Prior art
[prior-art]: #prior-art

OOP languages naturally support this via implicit or explicit virtual markers.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The case of `mut self`.

# Future possibilities
[future-possibilities]: #future-possibilities

I have to think if the above interacts negatively with async but it doesn't seem to on a first pass.