- Start Date: 2015-01-26
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Add keyword arguments to Rust in a backwards-compatible way.
This allows overloading that is safe in regards to type inference while being decided at compile time.

# Motivation

Allow another kind of argument in Rust. Current arguments distinguish themselves from other types by their order.
If there is no semantic reason for a certain order, the argument order in functions is basically arbitrary.
Consider a call like this:
```rust
window.addNewControl("Title", 20, 50, 100, 50, true);
```

First of all, the call tells nothing the user about what those values mean.
Secondly, their order is arbitrary and must be memorized. What I propose is that this call would rather look like:
```rust
window.addNewControl(
    title => "Title", 
    xPosition => 20, 
    yPosition => 50, 
    width => 100, 
    height => 50, 
    drawingNow => true);
```

While you might argue that this is more verbose, this is already the standard in the JavaScript ecosystem.
A lot of libraries in JavaScript actually have a convention with calling with associative arrays like:
```JavaScript
window.addNewControl({ xPosition: 20, yPosition: 50, width: 100, height: 5,
                 drawingNow: true });
```
If this wasn't a convenient pattern, nobody would bother to do it.

An additional benefit is that this leads to an easy implementation of optional parameters and optional parameters.
In languages like PHP, default parameters must come at the end of the function and must have a value.

You can have optional parameters in the beginning, middle and end of functions without any default value.
This means that this design allows for truly optional parameters without any run-time cost of using an `Option`.

# Detailed design
Currently Rust has

```rust
fn slice(&self, begin: usize, end: usize) -> &'a str
fn slice_from(&self, begin: usize) -> &'a str
fn slice_to(&self, end: usize) -> &'a str
```

This can be changed to

```rust
fn slice(&self, from => begin: usize, to => end: usize) -> &'a str
fn slice(&self, from => begin: usize) -> &'a str
fn slice(&self, to => end: usize) -> &'a str
```

Note that these are three different functions that have three different signatures.
The keywords a function accepts is part of its signature. You can call these functions like this:

```rust
foo.slice(from => 5); //equivalent to current foo.slice_from(5)
foo.slice(to => 9);   //equivalent to current foo.slice_to(9)
foo.slice(from => 5, to => 9);       //equivalent to current foo.slice(5, 9)
foo.slice(from => 9, to => 5);       //equivalent to current foo.slice(5, 9)
```

The trait looks like

```rust
Fn(&self, from => begin: usize, to => end: usize) -> &'a str
//which desugars to
Fn<({from: usize, to: usize}, &Self), &'a str>
```

So this is equivalent for using a `struct` to pass around your keyword parameters.
But now you are able to use the same function name several times as long as the keywords are different.

# Drawbacks

This is a more complicated design than just having default arguments and overloading.
Now there are both positional and keyword arguments and they interact with lifetimes, traits, and closures.

# Alternatives

A better design to the above function might be designing it like so:

```rust
let title = "Title";
let position = Position(20, 50);
let dimensions = Dimension(100, 50);
window.addNewControlDrawingNow(title, position, dimensions);
```

Now the function takes three parameters, and we assigned them meaningful names.
Instead of passing a boolean, we created functions that do different things.
They could be named something like `addNewControl` and `addNewControlDrawingNow`.

While this design is better, it still doesn't solve the problem of having to remember of the order.
Where do you put `dimensions`, `position`, and the `title`?
At least the compiler will now verify that the types are correct.
It is still up to the programmer to actually name those variables well, instead of the API specifying what the keywords should be.

If keyword arguments themselves are not implemented, then there's also the issue of overloading to enable better API design.
Another proposal would be to assign keywords to all current Rust code with their local names.
So something like:

```rust
foo.slice(begin => 5, end => 10);
```

This will solve the problem of the naming of parameters, but won't solve the problem of overloading.
Because this design this allows you to call this method like:

```rust
foo.slice(x, y);
```

This means that you potentially can't infer the types of the arguments if they were overloaded.
Another overload could add another `slice()` that takes a different type of parameter in the first slot.
What type would `x` be then?
Even though it has a different name, being able to call keyword arguments without keywords breaks overloading.

With mandatory keywords it is always known what type a certain keyword is.

# Unresolved questions

The trait desugaring is an addition based on the discussion on the internals board.
Could it come first (before the self parameter) or does it have to come last?

Another possibility is a different syntax for keywords. Maybe it would be better to unify it with the struct syntax.
So something like `foo.slice(begin: 5, end: 10)` because it desugars to a keyword struct anyway.

But what would the declaration look like?