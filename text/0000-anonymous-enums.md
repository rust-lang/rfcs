- Start Date: December 9, 2014
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add support for defining anonymous, enum-like types using `A | B`.

# Motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

### A File System like Program

Consider the following code:

```rust
/// All Files can be `stat`d
pub trait Stat { fn stat(&self) -> Stat; }

/// A FileObject can be read and written to.
pub struct File { ... }
impl Stat for File { ... }

/// Get a File object
pub fn open(&str) -> File { ... }
```

This is all good and we can use it to get the file objects. What if later
however we decide to expand our filesystem. Now we want to add a directory
type.

```rust
/// A DirectoryObject contains some number of files and directories.
pub struct Directory { ... }
impl Stat for Directory { ... }

/// What do we return here? It cannot be a Directory because we might be getting a File.
pub fn open(&str) -> ??? { ... }
```

We know that many consumers simply want to be able to check that a particular
FS Object exists, for which they need a `&Stat`, some others might want to
dispatch based on which type they get. For example a tool like [find
(1)](http://linux.die.net/man/1/find) might want to print out the names of
`Symlink`s and `File`s and for `Directory`s print their name and then recur on
the list of objects inside of them. One way to get around this would be to make
an enum which contains all of these types.

```rust
pub enum FileSystemObject {
    FileObject(File),
    DirectoryObject(Directory),
}

impl Stat for FileSystemObject {
    fn stat(&self) -> Stat {
        use FileSystemObject::*;
        match *self {
            FileObject(f) => f.stat(),
            DirectoryObject(d) => d.stat(),
        }
    }
}

pub fn open(&str) -> FileSystemObject { ... }
```

Now this basically works but what if later we decide that actually we wish to
add symlinks, which can point to either a directory, file or another symlink.
Well we can first add it to the `FileSystemObject` enum and stat and get:

```rust
pub struct Symlink { ... }
impl Stat for Symlink { ... }
pub enum FileSystemObject {
    FileObject(File),
    DirectoryObject(Directory),
    SymlinkObject(Symlink),
}

/// We want to be able to just use Stat, since it is common to everything it
/// would be annoying having to destructure this whole thing to get at it.
impl Stat for FileSystemObject {
    fn stat(&self) -> Stat {
        use FileSystemObject::*;
        match *self {
            FileObject(f) => f.stat(),
            DirectoryObject(d) => d.stat(),
            SymlinkObject(s) => d.stat(),
        }
    }
}
```

But now we realize that `Symlink`s and `Directory`s are very isomorphic to one
another, in that a `Directory` can be dereferenced to the zero or more element
list of its contents and so can a symlink. We decide to store this information
in a trait which both implement.

```rust
pub trait Listable { pub fn get_contents(&self) -> Vec<FileSystemObject>; }
impl Listable for Symlink { ... }
impl Listable for Directory { ... }
```

This is great but now we start having code which just wants to deal with
`Listable` types. Since a `File` is not `Listable` the way they do it is:

```rust
let obj = open(...);
let listable = match &obj {
    &SymlinkObject(ref s) => s as &Listable,
    &DirectoryObject(ref d) => d as &Listable,
    _ => { return Err(...); },
};
```

This works okay but what if we add another new `Listable` type? Suddenly we need
to go through all the client libraries that are looking for a simple `Listable`
and make sure to update them to include this new type in their match!
Furthermore this is rather ugly in the first case.

Under this proposal we would be able to write the `open` function like so:

```rust
pub fn open(&str) -> (Listable|Directory|File|Symlink) { ... }
```

Further any users of the library who simply want to use something with the
`Listable` bound would be able to do it as follows:

```rust
let obj = open(...);
match obj {
    list as Listable + ? => { list.get_contents() ... },
    _ => { return Err(...) }
}
```

### An example with errors

Another, perhaps easier to understand example could be:

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

Add a new notation for anonymous enums, `A | B`, called `union` types. This is best
explained via a small literate program:

```rust
struct A; struct B; struct C;
```

Unions, like `A | B` are normal types.

```rust
type AorB = A | B;
```

The notation is order independent, `A | B` is the same type as `B | A`.
In the same vein, multiple occurrences of `A | B`, even in different crates,
are semantically the same type.

```rust
type BorA = B | A;

let foo: AorB = A;
let bar: BorA = x;
```

### Traits and unions

Trait impls on unions follow the regular coherence rules as they apply to
tuples - at least one of the types in the union must be defined in the
same crate or the trait must be defined in the same crate.

```rust
impl Show for A|B {
    fn fmt(&self, f: &mut fmt::Formmatter) -> fmt::Result { write!(f, "we are an A or a B") }
}
```

### Matching Unions and Bounds

To disambiguate a union into one of its constituent types, we use `match`,
the same as with normal enums.

```rust
match x {
    B => println!("It's B!");
    A => println!("It's A!");
}
```

In order to prevent ambiguity one may not destructure anonymous union types. One
may, however do a checked cast and access them as their constituent types using
the `as` token to denote doing a checked cast.

```rust
struct X { x: int }
struct Y { y: float }

// ...

match z {
    x as X => { println!("x's value is {}", x.x); },
    y as Y => { println!("y's value is {}", y.y); },
}
```

In cases where the type system cannot prove that the types in a union are
mutually exclusive (for example, at least one bound is a trait) one will be
required to handle any cases of overlap in a match. For example:

```rust
trait Enter { fn say_hi(&self) -> &str; }
trait Leave { fn say_bye(&self) -> &str; }

trait Talker { fn talk(&self); }

impl Talker for Enter | Leave {
    fn talk(&self) {
        match *self {
            x as Enter         => { println!("hi-{}", x.say_hi()); },
            x as Leave         => { println!("bye-{}", x.say_bye()); },
            x as Enter + Leave => { println!("hi-{} and bye-{}", x.say_hi(), x.say_bye()); },
        }
    }
}
```

One may use a name without bounds or the standard `_` wildcard to denote default value.

```rust
let abc : (A|B|C) = ...;
match abc {
    a as A => { ... },
    x => { ... }, // The default case. x is (B | C | A + (B | C))
}
```

In the standard case, however the type system should be able to prove that most
or all compound bounds are impossible for example in the following.

```rust
struct X;
struct Y;
trait Z {}

match a {
    x  as X     => { ... },
    y  as Y     => { ... },
    z  as Z     => { ... },
    xz as X + Z => { ... },
    yz as Y + Z => { ... },
    // The following two are illegal since X and Y are structs and thus
    // these bounds are impossible to meet.
    xy  as X + Y     => { ... }, // Error: Unreachable
    xyz as X + Y + Z => { ... }, // Error: Unreachable
}
```

Of course a common goal might be to check if a single type, out of many in the
bound, is available. For this we can use union types as well. Note that we use
the `?` to indicate the union of all possible types, useful for getting out
something that is guaranteed to be some subset of the types.

```rust
trait A { ... }
trait B { ... }
trait C { ... }

let abc : A | B | C = ...;

match abc {
    ab        as A + B + ? => { ... },     // Is an A + B, and any or no other types.
                                           // The `?` is syntactic sugar for the set of all
                                           // possible types, here ? = (A|B|C)
    _ => { ... },                          // Default bounds. Everything not above.
}
```

### Declaration and Type Inference

Since the variants of a union type are not named, there is no explicit
instantiation syntax. Instead, types which are listed in a union are
implicitly coercible to the union type. They can also be converted using
`as` where it would be inconvenient to otherwise give a type hint.

The obvious syntax is ambiguous with bitwise XOR (`|`) for integers.
As a result, it must be disambiguated using `A as (A | B)`.

```rust
let x = A as (A | B);
```

### Automatic Trait Implementation

In a significant departure from the behavior of regular enums, if all of the
types in a union fulfill a certain bound, like `Copy` or `Show`, then the union type
*also* fulfills that bound.

```rust
fn is_static<T: 'static>() {}
is_static::<A | B>();
```

For bounds which imply methods, such as `Show`, the method is supplied by
simply unwrapping the data within the union through match and applying the
method.

```rust
let z = A as (A | B);
println!("{}", z); // uses the impl of Show for A
```

As an example, the above expands as

```rust
impl Show for A | B {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ref a    as A     => a.fmt(f),
            ref b    as B     => b.fmt(f),
            ref both as A + B => both.fmt(f),
        }
    }
}
```

Of course this expansion is done totally within the compiler, writing such an
implementation in standard rust should give a multiple implementation error.

### Methods on Unions

One is allowed to call only trait methods through union types. Specifically one
is only allowed to call methods of the intersection of all the types in the
union. Implementations of traits with incompatible type arguments are
considered to be disjoint.

```rust
#[deriving(Show)] struct X { ... }
impl ToOwned<A> for X { ... }
impl ToOwned<C> for X { ... }

#[deriving(Show)] struct Y { ... }
impl ToOwned<B> for Y { ... }
impl ToOwned<C> for Y { ... }

let a : X | Y = ...;

// Ok
let c : C = a.to_owned();

// Ok
println!("showing {}", a);

// Error: ToOwned<B> is not implemented by type X | Y because it is not
//        implemented by type X
let b = a.to_owned::<B>();
```

### Errors and the Type-Checker

We always flatten types as much as possible, so therefore the following
declarations are all equivalent:

```rust
type T1 = A | B | C;
type T2 = T1 | B;
type T3 = T1 | T2;
```

We do not allow a single type to appear multiple times in a direct union type
declaration as a way to prevent errors and enforce good style. The system will
need to be capable of handling this, however, to deal with type arguments.

```rust
// Not allowed
type X = A | B | A;

// Allowed
fn maybe_str<T>(x: T) -> T | &str { if is_full_moon() { x } else { "no full moon" } }
let x : &str = maybe_str::<&str>("argument");
```

Any type is coercible to a union of itself and any other type.

```rust
let x : Vec<uint | &'static str> = vec![1, 2, "hello", "goodbye"];
let x = vec![1 as (uint|&'static str), 2, "hello", "goodbye"];
```

It should throw an error if the unification of types requires a union and one
has not already been declared.

```rust
// Error: Expected: Vec<uint>, Found: Vec<uint|&'static str>
// Error: Did you mean: let x : Vec<uint|&'static str> = vec![1,2,"hello","goodbye"];
let x = vec![1, 2, "hello", "goodbye"];
```

# Drawbacks

It adds a new relatively complicated feature.

Adds a new special token `?` to the language. Furthermore the fact that it can
only be used in match specifications is somewhat surprising.

It is somewhat unintuitive that a value of type `A | B` could be both types at
once, and must be matched as such. Further the fact that this depends on
whether or not either type is a `trait` makes this potentially even more
confusing.

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

The syntax for checking whether a value is of a single specific type,
regardless of the other bounds on it is somewhat unintuitive.

It might not be possible to gaurentee that the **in memory** layout of two
semantically identical union types are the same. This would prevent transmuting
between union types.

The most obvious way to implement this would be to have the compiler generate a
standard enum which contains variants for all of the possible bounds, this
could lead to long compile times and large enums in cases where the types are
not tightly constrained. For example the following would require the compiler
to generate a 120-variant enum to accomadate any of the types being traits.

```rust
fn pick_one<A, B, C, D, E>(a: &A, b: &B, c: &C, d: &D, e: &E) -> &(A|B|C|D|E) { ... }
```

The `FromError` trait fullfills the most obvious use for this already. On the
other hand this can still be very useful in creating things such as file tree
representations.

# Alternatives

Keep the status quo, which is to define new library enums.

Introduce a new sugar for creating simple enums.

Allow implicit coercions between regular enums.

Keep the anonymous enum syntax but cut some of the behaviors
unique to it, such as allowing impls, making them order dependent,
not allowing implicit coercions, &c.

Only allow the anonymous enum syntax to be used with concrete, structure types.
Eliminating the most complicated parts of it. Unfortunately this also
eliminates its most useful features.

# Unresolved questions

How should this interact with type inference?

Should negative bounds be allowed in `match` statements? This would be another
compilicated feature to add, however without it the default match arm must be
implemented as a special case.

Should additional possible unioned traits be infered? For instance should the
following code be legal?

```rust
trait Awesome { ... }
struct A;
struct B;
struct C;

impl Awesome for A { ... }
impl Awesome for B { ... }

// ...

// Awesome is not in the bounds, but two of the structures implement it.
let abc : (A|B|C) = ...;
match abc {
    x as Awesome + ? => { println!("{} is AWESOME", x); }
    x => { println!("{} is Over-hyped", x); }
}
```

Should parenthesis be *required* around Union types? They are not required in
this document but it is almost always better in terms of reduced ambiguity.

Should we allow impls to be coerced into the approprate types? For example
should the following code be legal? If not how should we deal with cases where
the required signature includes multiple copies of the same type, as in the
last example below.

```rust
trait Printer {
    fn get_printable(&self) -> T | &str
}

impl Printer<A> for A {
    fn get_printable(&self) -> A { self }
}
impl Printer<A> for &str {
    fn get_printable(&self) -> &str { self }
}
impl Printer<B> for B {
    fn get_printable(&self) -> B { self }
}
impl Printer<&str> for &str {
    fn get_printable(&self) -> &str { self }
}
```

