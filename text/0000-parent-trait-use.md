- Feature Name: parent_trait_use
- Start Date: 2018-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow implementing functions, associated constants and associated types of parent traits inside child traits.

# Motivation
[motivation]: #motivation

This may have two kinds of effects: Better ergonomics for trait implementors and being able to change trait hierarchy by splitting types as a crate maintainer without breaking anything.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Assume there is a trait `A` which has parent traits `B` and `C`:
```
trait B {
    fn b();
}
trait C {
    fn c();
}
trait A: B+C {
    fn a();
}
```
Now you need to implement every of these traits for your new type `X` manually. But you only needs trait `A` and don't really care about `B` and `C`, maybe they are defined in some different crates, which you would need to depend on directly.
So it's possible to implement every function directly inside the child trait like this:
```
struct X;
impl A for X {
    fn a() {…}
    fn b() {…}
    fn c() {…}
}
```
But in order to allow this, trait `A` needs to be defined in a different way, like this:
```
trait A: use B+use C {
    fn a();
}
```
Now everyone, who implements trait `A` can also implement the used functions of trait `B` and `C`.
If at least one function, type or constant of `B` and `C` has the same name, they cannot be used both.
There is one exception for associated types.
A simple math type, which may be defined like this, will show that:
```
trait Math: use Add<Output=<Self as Math>::Output>+
            use Sub<Output=<Self as Math>::Output>+
            use Mul<Output=<Self as Math>::Output>+
            use Div<Output=<Self as Math>::Output> {
    type Output;
}
```
Then you can just use this trait in order to implement all ops of these traits.
You see, if you specify `Output` in all parent traits in angle brackets, the `Output` type is never used and you can even specify it yourself.

Now you are the maintainer of a crate. You defined a trait `A`, which everyone uses, that looks like this:
```
trait A {
    fn a();
    fn b();
    fn c();
}
```
You recognize, it may be useful to split parts of this traits into single traits with less functionality. You split some of the functions into seperate traits, which looks as the first example:
```
trait B {
    fn b();
}
trait C {
    fn c();
}
trait A: B+C {
    fn a();
}
```
Now every user, that implemneted `A`, now has to implement `B` and `C`, so it would break their crates when updating version.
The solution is the same as before:
```
trait A: use B+use C {
    fn a();
}
```

In order not to break anything, this change has to be recursive.
Someone may already use `A` as a using parent trait:
```
trait X: use A {}
```
This trait will need to use all functions, that are impmenetable inside `A`, so after the split of `A` it will also use functions of `B` and `C`.

This use case is probably more important than ergonomics.

When defining a trait, that uses it's parent traits, it will just expand to multiple trait definitions. If a trait is used, and at least one of it's functions is defined in this trait, it will define the trait. If multiple traits use the same trait, only one of them is allowed to define functions of that trait.
If types are defined in a supertrait, but are set using angle brackets, these types will be implictely set to the type in angle brackets after expanding.

Assuming you want to implement the previously mentioned `Math` trait for some type, you could write this:
```
type Number;
impl Math for Number {
    type Output = Number;
    fn add(self, rhs: Self) -> Self {…}
    fn sub(self, rhs: Self) -> Self {…}
}
```
This would expand to following:
```
type Number;
impl Add for Number {
    type Output = Number;
    fn add(self, rhs: Self) -> Self {…}
}
impl Math for Number {
    type Output = Number;
    fn sub(self, rhs: Self) -> Self {…}
}
impl Math for Number {
    type Output = Number;
}
```
You would still need to implement `Mul` and `Div`, else you will get errors, as if you wrote the expandet form directly. It's also possible to implement these traits seperately.

This won't affect existing code. It may be still preferable to define everything inside the traits, they are used as. Implementing a trait, that uses it's parent traits should only be used for important types.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This will be implemented by recursively searching for all names of traits, which are used by the current trait.
If one of the names exists at least twice, this will be reported as an error when the trait is defined.
If associated types are set in the used traits using angle bracket notation, they won't be counted, and will just defined as default value for the used traits, which are implemented in this way.
When implementing, the functions, associated types and constants will be defined in the matching traits.


# Drawbacks
[drawbacks]: #drawbacks

Everyone may implement multiple traits inside a single using trait, what may lead to confusion, which traits they are actually implementing.


# Rationale and alternatives
[alternatives]: #alternatives

Other syntax:

* `impl` keyword:
```
trait A: impl B+impl C {…}
```
* use the traits:
```
trait A: B+C {
    use B;
    use C;
    …
}
```
* use the functions directly: (using single elements may not be useful)
```
trait A: B+C {
    use B::b;
    use C::c;
    …
}
```

Automatically allow implementing items of parent traits in child triats, when there is no ambigouity.

# Unresolved questions
[unresolved]: #unresolved-questions

