- Feature Name: `import-trait-associated-functions`
- Start Date: 2024-03-19
- RFC PR: [rust-lang/rfcs#3591](https://github.com/rust-lang/rfcs/pull/3591)
- Rust Issue: [rust-lang/rust#134691](https://github.com/rust-lang/rust/issues/134691)

## Summary
[summary]: #summary

Allow importing associated functions and constants from traits and then using them like regular items. 

## Motivation
[motivation]: #motivation

There has for a long time been a desire to shorten the duplication needed to access certain associated functions, such as `Default::default`. Codebases like [Bevy](https://github.com/bevyengine/bevy/blob/7c7d1e8a6442a4258896b6c605beb1bf50399396/crates/bevy_utils/src/default.rs#L27) provide wrapper functions to shorten this call, and a previous, now-rejected, [RFC](https://github.com/rust-lang/rust/pull/73001) aimed to provide this function as part of the standard library. This RFC was rejected with a note that there is a desire for a more general capability to import any trait associated functions.

Additionally, if you pull in a crate like [num_traits](https://docs.rs/num-traits/latest/num_traits/), then this feature will allow access to numeric functions such as `sin` using the `sin(x)` syntax that is more common in mathematics. More generally, it will make calls to trait associated functions shorter without having to write a wrapper function.

Similarly, associated constants, which act much like constant functions, can be imported to give easier access to them.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Importing an associated functions from a trait is the same as importing a function from any module:
```rust
use Default::default;
```

Once you have done this, the function is made available in your current scope just like any other regular function.

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

You can also use this with trait methods (i.e. associated functions that take a `self` argument):

```rust
use num_traits::float::Float::{sin, cos}

fn eulers_formula(theta: f64) -> (f64, f64) {
    (cos(theta), sin(theta))
}
```

Importing an associated function from a trait does not import the trait. If you want to call `self` methods on a trait or `impl` it, then you can import the trait and its associated functions in a single import statement:

```rust
mod a {
    trait A {
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

Associated functions can also be renamed when they are imported using the usual `as` syntax:
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

You cannot import a parent trait associated function from a sub-trait:

```rust
use num_traits::float::Float::zero; // Error: try `use num_traits::identities::Zero::zero` instead.

fn main() {
    let x : f64 = zero();
    println!("{}",x);
}
```

Importing an associated constant is allowed too:
```rust
mod m {
    trait MyNumTrait {
        const ZERO: Self;
        const ONE: Self;
    }

    // Impl for every numeric type...
}

use m::MyNumTrait::ZERO;

fn f() -> u32 {
    ZERO
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When 

```rust
use Trait::item as m;
```
occurs, a new item `m` is made available in the value namespace of the current module. Any attempts to use this item are treated as using the associated item explicitly qualified. `item` must be either an associated function or an associated constant. As always, the `as` qualifier is optional, in which case the name of the new item is identical with the name of the associated item in the trait. In other words, the example:

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
use Trait::func as m;
m(x, y, z);
```
desugars to
```rust
Trait::func(x, y, z);
```

Additionally, the syntax
```rust
use Trait::{self, func};
```
is sugar for
```rust
use some_module::Trait;
use some_module::Trait::func;
```

The restriction on importing parent trait associated functions is a consequence of this desugaring, see https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=51bef9ba69ce1fc20248e987bf106bd4 for examples of the errors you get when you try to call parent trait associated functions through a child trait. We will likely want better error messages than this if a user tries to import a parent function.

Note that trait generics are handled by this desugaring using type inference. As above, given `Trait<T>`,
```rust
use Trait::func as m;
m(x, y, z);
```
desugars to
```rust
Trait::func(x, y, z);
```
which compiles if and only if `T` and `Self` can be inferred from the function call. For example, if `func` was
```
fn func(self, b: T, c: i32) {}
```
then `Trait<T>` would be inferred to be `<typeof(x) as Trait<typeof(y)>`. Generics on `Trait` are not directly specifiable when a function is called in this way. To call a function with explicit types specified you must use the usual fully qualified syntax.

## Drawbacks
[drawbacks]: #drawbacks

Calls to `default` are less explicit than calls to `Default::default` or to `T::default`, likewise for any other trait. Some users may see this lack of explicitness as bad style.

To expand on this, [the book](https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html#creating-idiomatic-use-paths) currently recommends that functions should be called using their parent module's name:
> Although both Listing 7-11 and 7-13 accomplish the same task, Listing 7-11 is the idiomatic way to bring a function into scope with use. Bringing the function’s parent module into scope with use means we have to specify the parent module when calling the function. Specifying the parent module when calling the function makes it clear that the function isn’t locally defined while still minimizing repetition of the full path.

This recommendation makes the most sense when there is a possibility of ambiguity in the mind of the reader. For example, a function like `sin` is unlikely to be ambiguous, because there is only one mathematical function of that name. If a codebase is likely to be making use of multiple different implementations of `sin`, then it makes more sense to require specifically naming the one you are going to use. Similar considerations apply to traits like `Default::default`, or more generally in cases like `Frobnicator::frobnicate`.

Because of this context sensitivity, we should allow developers to choose when removing the extra context makes sense for their codebase.

Another drawback mentioned during review for this RFC was that this adds more complication to the name resolution rules. On an implementation side, I am assured that this feature is straightforward to implement in rustc. From a user perspective, the name lookup rules for the function name are exactly the same as those used to look up any other function name. The lookup rules used to resolve the `impl` are also exactly the same ones used for non-fully qualified trait function calls. There is no fundamentally new kind of lookup happening here, just a remixing of existing lookup rules.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why is this design the best in the space of possible designs?

This design is minimalist, it adds no extra syntax, instead providing a natural extension of existing syntax to support a feature that is frequently requested. Users might very well already expect this feature, with this exact syntax, to be present in Rust, and surprised when it isn't.

### What other designs have been considered and what is the rationale for not choosing them?

In [Zulip](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Writing.20an.20RFC.20for.20.60use.20Default.3A.3Adefault.60/near/427795694), there was some discussion of whether `use Trait::func` should bring `Trait` into scope. There are three possibilities:

1. It does not - this may be unexpected, but maybe not
2. It does - then `value.other_func_from_the_same_trait()` will work as well, this may be unexpected too
3. It does, but only for `func`, that's something new for the language (need new more fine-grained tracking of traits in scope)

Option 1 is what is proposed here. It has the simplest semantics, and I believe it best matches the user intent when they import an associated function; the desire is to make that function available as-if it were a regular function. Furthermore, it is more minimalist than the other two options in the sense that you can get to option 2 simply by importing the trait also. Option 3 seems like extra complexity for almost no added value.

We considered allowing `use Trait::parent_method`, but decided against it, as you can always explicitly import from the parent instead.

### What is the impact of not doing this?

Users of the language continue to create helper functions to access associated with regular function syntax. More specifically, each such instance requires a minimum of three lines when using normal rust formatting, corresponding to the following example:
```rust
fn my_trait_func<T: MyTrait>(args) -> ret {
    MyTrait::my_trait_func(args)
}
```
Such code is boilerplate that serves nobody's time to have to write repeatedly.

### If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

A library solution has already been rejected for this. This solves the same problem as a library solution in a much more general way, that doesn't require adding new library functions every time we want shorthand access to trait function names.

## Prior art
[prior-art]: #prior-art

As mentioned in [motivation], there was a rejected [RFC](https://github.com/rust-lang/rust/pull/73001) for adding a function `std::default::default` to make calling `Default::default` less repetitive. This RFC was rejected, with a desire to see something like what this RFC proposes replace it.

[This issue](https://github.com/rust-lang/rfcs/issues/1995) also lists some further motivation for this feature.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is specifying this in terms of desugaring sufficient to give the desired semantics?

## Future possibilities
[future-possibilities]: #future-possibilities

This RFC does not propose the ability to import `Type::method` where `method` is contained in an `impl` block. Such a feature would be a natural extension of this work, and would enable numeric features like that discussed in [motivation] without the need for the [num_traits](https://docs.rs/num-traits/latest/num_traits/) crate. This feature is not proposed in this RFC since initial investigations revealed that it would be [difficult](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Writing.20an.20RFC.20for.20.60use.20Default.3A.3Adefault.60/near/427804375) to implement in today's rustc.

If we add a compatibility mechanism to implement a supertrait method when implementing its subtrait, without having to separately implement the supertrait (such that a new supertrait can be extracted from a trait without breaking compatibility), we would also need to lift the limitation on using a supertrait method via a subtrait.
