- Feature Name: `assume_bound`
- Start Date: 2025-04-21
- RFC PR: [rust-lang/rfcs#3802](https://github.com/rust-lang/rfcs/pull/3800)
- Rust Issue: N/A

# Summary
[summary]: #summary

This feature allows to assume trait bounds on generics so that the caller don't has to proof them pre-monomorph.

# Motivation
[motivation]: #motivation

I am currently writing a lot of generic magic again.
To be more concrete, I am making a framework where lots of functions with generic parameters call eachother.
There, I pass some `impl Key` values, which hold an assoicated type and proof that the generic, which is passed down, implements `Has<T>`, where T is that associated bound.
Now I don't really want to specify that requirement on every function, as that is cumbersome and hard to maintain. 
Thus I propose to let me have my unsafe fun to assume trait bounds are fulfilled in the places I am sure about.

Another use case is for complex higher ranked bounds, where a small helper function could be used to hint the compiler that a trait is really implemented.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When implementing a function with a `where` clause, like this one:
```rs
pub fn print<T>(val: T)
where
    T: Debug
{ .. }
```
and you want to call that from a less restricted generic
```rs
fn less_restricted<T>(val: T) {
    print(val); // error[E0277]: `T` doesn't implement `Debug`
}
```
may it be from a library where you can't restrict it further but know that you only pass in correct types, you could assume the `T: Debug` condition on the upper function:
```rs
pub fn print<T>(val: T)
where
    #[unsafe(assume)] T: Debug
{ .. }
```
This makes `T` still behave like having `Debug` in the function body, but that isn't the case for the caller. \
There, it skips the check and just assumes the condition is true.

You may of course not want to change the `print` function, so you could also make a small util function just for the `less_restricted` call, like so:
```rs
fn less_restricted<T>(val: T) {
    fn print_assumed<T>(val: T)
    where
        #[unsafe(assume)] T: Debug
    {
        print(val);
    }

    print_assumed(val); // works fine
}
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`assume`d bounds are just skipped during bounds check and we trust the user.

Later, the compiler could assist with some wrong conditions, like if for example I would pass something in here which doesn't implement `Debug`, the compiler could tell me post-monomorph that this assumed trait bound is not fulfilled for that _specific_ type. But you shouldn't 100% depend on this, as for example lifetimes aren't preserved up to that stage, so any lifetime-dependant condition is completely unchecked, thus making it `unsafe`.  

# Drawbacks
[drawbacks]: #drawbacks

Using it is a pretty risky thing and it could also lead some people to prefer using that instead of writing one or two more bounds, but having it there as unsafe is still important I think.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

While it makes signatures higher up the call stack not as strict as they could be, I think having this as a possibility is still important, especially for more complex higher ranked bounds.

# Prior art
[prior-art]: #prior-art

See the [discussion on the internals forum](https://internals.rust-lang.org/t/giving-generics-traits-via-unsafe-code/22753)

# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Future possibilities
[future-possibilities]: #future-possibilities

In a "perfect" world where the compiler could access lifetimes in monomorph, one could remove the `unsafe`, tho that would still make the signatures higher up the call stack less strict.