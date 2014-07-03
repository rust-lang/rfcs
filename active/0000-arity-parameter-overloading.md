- Start Date: 2014-07-03
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC proposes to add arity-based parameter overloading to Rust.

# Motivation

Currently in Rust there are a lot of functions that do the same thing, but take a different number of parameters.
The current design forces those functions to have different names.
This means that sometimes it's harder to look up function names because they are completely unrelated.

# Detailed design

Java has a very complicated overloading design that includes overloading by static types.
Overloading on types mixed with type inference might be very confusing.
However, overloading based on arity is very simple and clear.
Nobody will be confused by which method is being called when they differ by how many arguments they have.

```rust
fn concat(&self) -> String {
  ...
}

fn concat(&self, sep: &str) -> String {
  ...
}

// compile error, because the first parameter's type
// doesn't match the type of `concat` already declared.
fn concat(&mut self, sep: &str) -> String {
  ...
}

// compile error, because the second parameter's type
// doesn't match the type of `concat` already declared.
fn concat(&self, number: int) -> String {
  ...
}
```

So `to_str_radix(&self, radix: uint) -> String` can be now written as `to_str(&self, radix: uint) -> String` while
`to_str(&self) -> String` still exists. This will let Rust get rid of the sheer multitude of functions that only
 differ by a few parameters like `split` and `splitn`.
 
Default arguments almost solve this problem, but they don't solve the problem with `unwrap` and `unwrap_or`.
Arity-based overloading allows you to have `unwrap(self, default: T) -> T` as well as `unwrap(self) -> T`.

This also allows you to return a different type. Again, these are different functions with the same name.

```rust
fn split<Sep: CharEq>(&self, sep: Sep) -> CharSplits<'a, Sep>
fn split<Sep: CharEq>(&self, sep: Sep, count: uint) -> CharSplitsN<'a, Sep>
```


# Drawbacks

Compared to default arguments, it is much more verbose and gives more power to the user.

1) Lets you return a different type
2) Lets you omit arguments completely for a different implementation

However, as you can see from the examples, `to_str`/`to_str_radix` only requires default arguments to be combined into one function.
`split`/`splitn` and `unwrap`/`unwrap_or` require overloading to be combined. One could make an argument that default arguments have
a better syntax and are less verbose. Implementing default arguments instead might be better.

# Alternatives

The aforementioned default arguments are a strong alternative since it's a lighter syntax.

Current APIs have three slice functions:

```rust
fn slice(&self, begin: uint, end: uint) -> &'a str
fn slice_from(&self, begin: uint) -> &'a str
fn slice_to(&self, end: uint) -> &'a str
```

This proposal does not let you have

```rust
fn slice(&self, begin: uint, end: uint) -> &'a str
fn slice(&self, begin: uint) -> &'a str
fn slice(&self, end: uint) -> &'a str
```

This is because Rust does not support keyword arguments. You can't distinguish between a beginning and an end.
If Rust did support keyword arguments you could call those functions like this:

```rust
foo.slice(begin => 5); //equivalent to current foo.slice_from(5)
foo.slice(end => 9);   //equivalent to current foo.slice_to(9)
foo.slice(begin => 5, end => 9);       //equivalent to current foo.slice(5, 9)
foo.slice(end => 9, begin => 5);       //equivalent to current foo.slice(5, 9)
```

Overloading on keywords a la Smalltalk is the most powerful and allows the most freedom in API design.
However, it should be left to another RFC since it is a separate idea from default arguments and overloading altogether.

# Unresolved questions

Would it be beneficial to implement both overloading and default arguments?
In a lot of cases, you want just default arguments, like in the case of `to_str` where you just want to write 
`fn to_str(&self, radix = 10u) -> String` or `fn to_str(&self, radix: uint = 10u) -> String` without type inference.

Having to write two type signatures like

```rust
fn to_str(&self, radix: uint) -> String
fn to_str(&self) -> String
```

seems like it is too verbose even if overloading is strictly more powerful.
