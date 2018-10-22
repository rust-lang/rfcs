- Feature Name: flat_map as an alias for and_then
- Start Date: 2018-10-22
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

This feature simply adds `flat_map` as an alias for `and_then` for monadic types such as `Option` and `Result`

# Motivation
[motivation]: #motivation

Coming from a functional background, having two different function names for what's essentially the same operation can be a bit confusing.
In order to alleviate some of that, I propose to make `flat_map` available as a sort of "flattened mapping" available to other monadic types available in Rust.

Beyond giving the programmer less mental overhead, it also enables the creation of certain macros that generically work on all types that implement `map` and `flat_map`,
such as -- but not limited to -- a Haskell-like `do` notation:

```
mdo! {
  a <- vec.get(0);
  b <- vec.get(1);
  yield a + b;
}
```

which would be the equivalent of writing

```rust
vec.get(0).flat_map(|a| vec.get(1).map(|b| a + b)); 
```

While this looks similar to the `?` notation already present in Rust, it
1. Does not foece early returns
2. Does not require the user to introduce a function to deal with it
3. Would work on iterators as well as `Option`/`Result` due to their shared API.

Adding this could also ease the future transition into using higher kinded types, as then the functions would already be in place and integrated into the code base.

Reason for making it an alias, and not a rename is to prevent breaking existing code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Let's set up a simple example that uses flat mapping.
Imagine for a moment that you have defined a function called `safe_divide`, that returns an `Option`.

If the right-hand argument is 0, you're dividing by 0, which is disallowed, so we return `None`; otherwise we return `Some(res)` carrying our result.
```rust
fn safe_divide(a: f32, b:f32) -> Option<f32> {
  if b == 0.0 { None } else { Some(a / b) }
}
```
If we were to simply chain multiple `safe_divide`s, we'd get some pretty funky results:
```rust
let res = safe_divide(1.0, 3.0).map(|x| safe_divide(1.5, x));
println!("{:?}", res); // prints: Some(Some(4.5))
```
However, if we do a flat mapping on it, it behaves as normal:
```rust
let res = safe_divide(1.0, 3.0).flat_map(|x| safe_divide(1.5, x));
println!("{:?}", res); // prints Some(4.5)
```
letting us chain it to our heart's content.

Now imagine having to split an email into its constituent parts, splitting the string on `@` and `.`.

You might be tempted to do this:
```rust
let email = "foo@example.com";
    
let res: Vec<Vec<_>> = email.split('@').map(|s| s.split('.').collect()).collect();

println!("{:?}", res); // prints: [["foo"], ["example", "com"]]
```
But not only does that result in a nested `Vec`, you also have to collect twice! (yikes!)

A much nicer solution would be to simply use a flat map:
```rust
let email = "foo@example.com";

let res: Vec<_> = email.split('@').flat_map(|s| s.split('.')).collect();

println!("{:?}", res); // prints: ["foo", "example", "com"]
```
Not only is the code cleaner, but so is the type and the output.

You can think of flat mapping as an escape hatch for avoiding nested result types that might occur any time you'd otherwise want to use `map`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

It's a different name for `and_then`. The simplest implementation would likely to just have `flat_map` call `and_then` internally to avoid reinventing the wheel.

# Drawbacks
[drawbacks]: #drawbacks

There might be cases where a new programmer might ask themselves why you'd ever need _two_ functions that do the exact same thing.

There might also be some clashing with some existing libraries that try to implement their own universal `flat_map`. 
Though it is unclear if those libraries didn't call it `bind` or similar, to avoid similar issues.

# Rationale and alternatives
[alternatives]: #alternatives

A different approach to this would be to add `and_then` as an alias for `flat_map` inside `Iterator`; in which case the above arguments still hold, just with a different function name.
I believe an uniform API is the best way to go.

# Prior art
[prior-art]: #prior-art

Both Haskell and Scala use this to great effect.
Haskell in the above-mentioned `do` notation, and Scala with its `for` comprehensions, which both leverage the fact that all monadic types have a common interface in the form of `fmap` and `>>=` for Haskell and `map` and `flatMap` for Scala.

So while higher kinded types might not yet be possible in Rust; the use of macros could certainly simulate this effect.
