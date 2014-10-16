- Start Date: October 15, 2014
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add support for defining anonymous, enum-like types using `A | B`.

# Motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

Consider the following code:

```rust
pub struct ErrorX;
pub struct ErrorY;

pub fn produce_error_x() -> ErrorX { ErrorX }
pub fn produce_error_y() -> ErrorY { ErrorY }

// One error type, so all is good.
pub fn some_operation() -> Result<(), ErrorX> {
    let x = try!(produce_error_x());
    let x1 = try!(produce_error_x());
    Ok(())
}

// Now we want to do operations which can produce different errors. Problem.
pub fn some_other_operation() -> Result<(), ??> {
    let x = try!(produce_error_x());
    let y = try!(produce_error_y());
    Ok(())
}
```

The above code will not compile, since `some_other_operation` wants to "throw"
two different error types. Our current solution to this problem is to create
a custom enum, add variants for the two error types, write a lifting function,
then return the enum.

That code looks like this:

```rust
pub struct ErrorX;
pub struct ErrorY;

pub enum LibError {
    X(ErrorX),
    Y(ErrorY)
}

impl LibError {
    // In this simplified example, these methods are not really necessary,
    // as construction is simple, but in many real usage sites, lifting
    // can be complex.
    pub fn lift_x(x: ErrorX) -> LibError { X(x) }
    pub fn lift_y(y: ErrorY) -> LibError { Y(y) }
}

pub fn produce_error_x() -> ErrorX { ErrorX }
pub fn produce_error_y() -> ErrorY { ErrorY }

pub fn some_other_operation() -> Result<(), LibError> {
    let x = try!(produce_error_x().map_err(|e| LibError::lift_x(e)));
    let y = try!(produce_error_y().map_err(|e| LibError::lift_y(e)));
    Ok(())
}
```

Besides introducing an extremely large amount of boilerplate for such a simple
thing, this approach both does not scale well to many error types and introduces
unnecessary ambiguity in the return type of functions like `some_other_operation`.

If we later added many more error types to our library, not only would we
have to add many more lifting functions, but function like
`some_other_operation`, which can only error in one of two ways, now have a
type which says they can fail in a large number of ways.

Under this proposal, the above code could instead be written like so:

```rust
pub struct ErrorX;
pub struct ErrorY;

pub fn some_other_operation() -> Result<(), ErrorX | ErrorY> {
    let x = try!(produce_error_x());
    let y = try!(produce_error_y());
    Ok(())
}
```

Which is much shorter, includes virtually no boilerplate, and is much more
specific in defining which errors `some_other_operation` is allowed to produce.

The `A | B` is deep syntactical sugar for an anonymous enum type, which is
roughly equivalent to creating a new enum type that contains `A` and `B` as
variants, but also has other additional features, detailed below.

# Detailed design

Add a new notation for anonymous enums, `A | B`, called `join` types. This is best
explained via a small literate program:

```rust
struct A; struct B; struct C;
```

Joins, like `A | B` are normal types.

```rust
type AorB = A | B;
```

The notation is order independent, `A | B` is the same type as `B | A`.
In the same vein, multiple occurrences of `A | B`, even in different crates,
are the same type.

As a result of this, no trait impls are allowed for join types.

```rust
type BorA = B | A;

let foo: AorB = A;
let bar: BorA = x;
```

To disambiguate a join into one of its constituent types, we use `match`,
the same as with normal enums.

```rust
match x {
    B => println!("It's B!");
    A => println!("It's A!");
}
```

Since the variants of a join type are not named, there is no explicit
instantiation syntax. Instead, types which are listed in a join are
implicitly coercable to the join type. They can also be converted using
`as` where it would be inconvenient to otherwise give a type hint.

```rust
let x = A as A | B;
```

In a significant departure from the behavior of regular enums, if all of the
types in a join fulfill a certain bound, like `Copy`, then the join type
*also* fulfills that bound.

```rust
fn is_static<T: 'static>() {}
is_static::<A | B>();
```

For bounds which imply methods, such as `Show`, the method is supplied by
simply unwrapping the data within the join through match and applying the
method.

```rust
let z = A as A | B;
println!("{}", z); // uses the impl of Show for A
```

As an example, the above expands as 

```rust
impl Show for A | B {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ref a @ A => a.fmt(f),
            ref b @ B => b.fmt(f)
        }
    }
}
```

Join types are represented in memory exactly like their equivalent enums.

For instance, `A | B` is represented the same as `enum AorB { a(A), b(B) }`.

# Drawbacks

It adds a new relatively complicated feature.

If you use `A | B` as a return type, especially for errors, adding a new
source of failure changes the type. This is problematic because this means
adding a new source of error you must cause a semver-breaking-change.

However, this is mitigated by the fact that changing possible errors of a
function can still be backwards incompatible, even if you are just returning
existing variants of an existing enum that the function just didn't return
before. That will still break code that looked like:

```rust
match some_operation() {
    Err(Variant1) | Err(Variant2) => {},

    // some_operation is documented to only throw Variants 1 and 2, not 3 or 4
    _ => unreachable!()
};
```

This proposal would make those assumptions encoded in the type system, which
means code like the above breaks early, but also causes other patterns to
break where they wouldn't in the past.

It introduces a new idea of an "anonymous type", since the concept does
not exist in Rust right now and all types have names or are, in the case
of unboxed closures, generated and interacted with through a trait.

# Alternatives

Keep the status quo, which is to define new library enums.

Introduce a new sugar for creating simple enums.

Allow implicit coercions between regular enums.

Keep the anonymous enum syntax but cut some of the behaviors
unique to it, such as allowing impls, making them order dependent,
not allowing implicit coercions, &c.

# Unresolved questions

Which methods are you allowed to call through a join type?

One possibility is allowing only methods that are object safe, that is
they do not mention the `Self` type and are not generic, but it's possible
that with some ingenuity we could get away with both.

How should this interact with type inference?

Should this: `let x = vec![1u, "hello", 7i]` compile with
`x: Vec<uint | &'static str | int>` or be rejected without further
annotations? Should you generally be able to coerce to a join type
without explicit annotation that you want a join type somewhere in
the program?

If so, that implies a large change in the way types are inferred
and could be a source of confusion.

