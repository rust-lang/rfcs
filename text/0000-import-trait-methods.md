- Feature Name: `import-trait-methods`
- Start Date: 2024-03-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow importing methods from traits and then using them like regular functions.

# Motivation
[motivation]: #motivation

There has for a long time been a desire to shorten the duplication needed to access certain methods, such as `Default::default`. Codebases like [Bevy](https://github.com/bevyengine/bevy/blob/7c7d1e8a6442a4258896b6c605beb1bf50399396/crates/bevy_utils/src/default.rs#L27) provide wrapper methods to shorten this call, and a previous, now-rejected, [RFC](https://github.com/rust-lang/rust/pull/73001) aimed to provide this method as part of the standard library. This RFC was rejected with a note that there is a desire for a more general capability to import any trait method.

Additionally, if you pull in a crate like [num_traits](https://docs.rs/num-traits/latest/num_traits/), then this feature will allow access to numeric methods such as `sin` using the `sin(x)` syntax that is more common in mathematics. More generally, it will make calls to trait methods shorter without having to write a wrapper function.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Importing a method from a trait is the same as importing a method from any module:
```rust
use Default::default;
```

Once you have done this, the method is made available in your current scope just like any other regular function.

```rust
struct S {
    a: HashMap<i32, i32>,
}

impl S {
    fn new() -> S {
        S {
            a: default()
        }
    }
}
```

You can also use this with trait methods that take a `self` argument:

```rust
use num_traits::float::Float::{sin, cos}

fn eulers_formula(theta: f64) -> (f64, f64) {
    (cos(theta), sin(theta))
}
```

Importing a method from a trait does not import the trait. If you want to call `self` methods on a trait or `impl` it, then you can import the trait as well as methods in the trait:

```rust
mod a {
    pub trait A {
        fn new() -> Self;
        fn do_something(&self);
    }
}

mod b {
    use super::a::A::{self, new}

    struct B();

    impl A for B {
        fn new() -> Self {
            B()
        }

        fn do_something(&self) {
        }
    }

    fn f() {
        let b: B = new();
        b.do_something();
    }
}
```

Trait methods can also be renamed when they are imported using the usual `as` syntax:
```rust
use Default::default as gimme

struct S {
    a: HashMap<i32, i32>,
}

impl S {
    fn new() -> S {
        S {
            a: gimme()
        }
    }
}
```

You can also import a parent trait method from a sub-trait:

```rust
use num_traits::float::Float::zero;

fn main() {
    let x : f64 = zero();
    println!("{}",x);
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When 

```rust
use Trait::method as m;
```
occurs, we first find the supertrait in which `method` occurs. If it occurs in multiple supertraits, or in the trait and in a supertrait, then error.
A new item `m` is made available in the function namespace of the current module. Any attempts to call this item are treated calling the (super-)trait method explicitly qualified. As always, the `as` qualifier is optional, in which case the name of the new item is identical with the name of the method in the trait. 

In other words, the example:

```rust
use Default::default;

struct S {
    a: HashMap<i32, i32>,
}

impl S {
    fn new() -> S {
        S {
            a: default()
        }
    }
}
```
desugars to 
```rust
struct S {
    a: HashMap<i32, i32>,
}

impl S {
    fn new() -> S {
        S {
            a: Default::default()
        }
    }
}
```
And a call
```rust
use Trait::method as m;
m(x, y, z);
```
desugars to
```rust
Trait::method(x, y, z);
```

Additionally, the syntax
```rust
use some_module::Trait::self;
```
is sugar for
```rust
use some_module::Trait;
```

Finally, given traits 
```rust
trait Super {
    fn f();
}

trait Sub : Super {
}
```
the usage
```rust
use module::Sub::f;
f();
```
desugars to
```rust
use module::Sub::f;
Super::f();
```
**not** `Sub::f();` as that desugaring will cause compiler errors, see https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=51bef9ba69ce1fc20248e987bf106bd4.

# Drawbacks
[drawbacks]: #drawbacks

Calls to `default` are less explicit than calls to `Default::default` or to `T::default`, likewise for any other trait. Some users may see this lack of explicitness as bad style.

To expand on this, [the book](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html#creating-idiomatic-use-paths) currently recommends that methods should be called using their parent module's name:
> Although both Listing 7-11 and 7-13 accomplish the same task, Listing 7-11 is the idiomatic way to bring a function into scope with use. Bringing the function’s parent module into scope with use means we have to specify the parent module when calling the function. Specifying the parent module when calling the function makes it clear that the function isn’t locally defined while still minimizing repetition of the full path.

This recommendation makes the most sense when there is a possibility of ambiguity in the mind of the reader. For example, a function like `sin` is unlikely to be ambiguous, because there is only one mathematical function of that name. If a codebase is likely to be making use of multiple different implementations of `sin`, then it makes more sense to require specifically naming the one you are going to use. Similar considerations apply to traits like `Default::default`, or more generally in cases like `Frobnicator::frobnicate`.

Because of this context sensitivity, we should allow developers to choose when removing the extra context makes sense for their codebase.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?

This design is minimalist, it adds no extra syntax, instead providing a natural extension of existing syntax to support a feature that is frequently requested. Users might very well already expect this feature, with this exact syntax, to be present in Rust, and surprised when it isn't.

## What other designs have been considered and what is the rationale for not choosing them?

In [Zulip](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Writing.20an.20RFC.20for.20.60use.20Default.3A.3Adefault.60/near/427795694), there was some discussion of whether `use Trait::method` should bring `Trait` into scope. There are three possibilities:

1. It does not - this may be unexpected, but maybe not
2. It does - then `value.other_method_from_the_same_trait()` will work as well, this may be unexpected too
3. It does, but only for method, that's something new for the language (need new more fine-grained tracking of traits in scope)

Option 1 is what is proposed here. It has the simplest semantics, and I believe it best matches the user intent when they import a trait method; the desire is to make that method available as-if it were a regular function. Furthermore, it is more minimalist than the other two options in the sense that you can get to option 2 simply by importing the trait also. Option 3 seems like extra complexity for almost no added value.

An earlier version of this RFC proposed not allowing `use Trait::super_trait_method`. This was changed because comments indicated this usecase would be common.

## What is the impact of not doing this?

Users of the language continue to create helper methods to access trait methods with regular function syntax. More specifically, each such instance requires a minimum of three lines when using normal rust formatting, corresponding to the following example:
```rust
fn my_trait_method<T: MyTrait>(args) -> ret {
    MyTrait::my_trait_method(args)
}
```
Such code is boilerplate that serves nobody's time to have to write repeatedly.

## If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

A library solution has already been rejected for this. This solves the same problem as a library solution in a much more general way, that doesn't require adding new library methods every time we want shorthand access to trait method names.

# Prior art
[prior-art]: #prior-art

As mentioned in [motivation], there was a rejected [RFC](https://github.com/rust-lang/rust/pull/73001) for adding a method `std::default::default` to make calling `Default::default` less repetitive. This RFC was rejected, with a desire to see something like what this RFC proposes replace it.

[This issue](https://github.com/rust-lang/rfcs/issues/1995) also lists some further motivation for this feature.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is specifying this in terms of desugaring sufficient to give the desired semantics?

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC does not propose the ability to import `Type::method` where `method` is contained in an `impl` block. Such a feature would be a natural extension of this work, and would enable numeric features like that discussed in [motivation] without the need for the [num_traits](https://docs.rs/num-traits/latest/num_traits/) crate. This feature is not proposed in this RFC since initial investigations revealed that it would be [difficult](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Writing.20an.20RFC.20for.20.60use.20Default.3A.3Adefault.60/near/427804375) to implement in today's rustc.
