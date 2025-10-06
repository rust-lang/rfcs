- Feature Name: `ignore_derive`
- Start Date: 2025-10-06
- RFC PR: [rust-lang/rfcs#3869](https://github.com/rust-lang/rfcs/pull/3869)
- Rust Issue: [rust-lang/rust#3869](https://github.com/rust-lang/rust/issues/3869)

# Summary
[summary]: #summary

The `#[ignore]` attribute can now be applied to fields.
Its purpose is to tell derive macros to skip the field when generating code.

```rust
#[derive(Clone, PartialEq, Eq, std::hash::Hash)]
struct User {
    #[ignore(PartialEq, std::hash::Hash)]
    //       ^^^^^^^^^  ^^^^^^^^^^^^^^^
    //       traits that will ignore this field
    name: String,
    #[ignore(PartialEq, std::hash::Hash)]
    age: u8,
    id: u64
}
```

For the above struct `User`, derives `PartialEq` and `Hash` will ignore the `name` and `age` fileds.
Code like this is generated:

```rust
impl Clone for User {
    fn clone(&self) -> User {
        User {
            name: self.name.clone(),
            age: self.age.clone(),
            id: self.id.clone(),
        }
    }
}

impl PartialEq for User {
    fn eq(&self, other: &User) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

impl std::hash::Hash for User {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) -> () {
        std::hash::Hash::hash(&self.id, state)
    }
}
```

# Motivation
[motivation]: #motivation

It's common to want to exclude one or more fields when deriving traits such as `Debug` or `Deserialize`.
To do this, you currently need to completely abandon the `derive` and instead implement the trait manually.

Manually implementing the trait is much more error-prone than letting the derive do it for you.
For example, when a new field is added, it's possible to forget to change all of your implementations.

This is why it's idiomatic to instead `derive` traits when it's possible to do so. But this deriving isn't possible if you need to ignore a field.
Common use-cases of ignoring fields when implementing traits include:

- Types that are high-level wrappers around other types, which may include some additional metadata but ultimately delegate most trait
  implementations to the inner type. An example is an identifier in programming languages:

  ```rust
  /// Identifier for a variable, function or trait
  #[derive(PartialEq, Eq)]
  struct Ident {
      /// Location of the identifier in the source code
      span: Span,
      /// Value of the identifier
      #[ignore(PartialEq, Eq)]
      value: String
  }
  ```
  
- Minor improvements such as skipping irrelevant fields in a `Debug` derive (like fields of type `PhantomData`) derive will become
  easy enough for people to just do. Currently, the effort required to maintain a manual implementation is too high -
  so people just don't bother with it in most cases.

- Security: Data with sensitive fields like AWS s3 `access_key_id` and `secret_access_key` that **must** be skipped for `Debug` implementations is forced to be implemented manually - leading to a significant increase in boilerplate

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When using `#[derive]`, you can apply `#[ignore]` to fields:

```rust
#[derive(Clone, PartialEq, Eq, std::hash::Hash)]
struct User {
    #[ignore(PartialEq, std::hash::Hash)]
    name: String,
    #[ignore(PartialEq, std::hash::Hash)]
    age: u8,
    id: u64
}
```

The `#[ignore]` receives paths to a subset of the derive macros applied to the item.

It is invalid for `#[ignore]` to receive a derive that isn't applied to the current item:

```rust
#[derive(Clone)]
struct Foo {
    #[ignore(Clone, PartialEq)]
    foo: String,
}
```

In the above example, `Foo` derives `Clone` but not `PartialEq` - so passing `PartialEq` to `ignore` is disallowed.

## From the perspective of a `derive` macro

When a derive macro such as `#[derive(std::hash::Hash)]` is applied to an item like a `struct`:

- Any `#[ignore]` attributes that mention the derive itself, in this case `std::hash::Hash`, will be a part of the `TokenStream`
  that the `derive` macro receives - **with the list of derives removed**. The derive macro has no idea what other derives ignore
  this field, it just knows that it should ignore it.

  **Example:** `std::hash::Hash` will see `#[ignore] field: ()` when the input contains `#[ignore(std::hash::Hash, Clone)] field: ()`

- If the `#[ignore]` attribute **does not** mention the derive, then the attribute is removed completely from the macro's input `TokenStream`.
  The derive macro doesn't know that other derives ignore this field.

  **Example:** `Clone` will see just `field: ()` when the input contains `#[ignore(std::hash::Hash, Clone)] field: ()`

### Example

In the below example:

- `Clone` will ignore fields `bar` and `baz`
- `std::hash::Hash` will ignore fields `foo` and `baz`

```rust
#[derive(std::hash::Hash, Clone)]
struct Foo {
    #[ignore(std::hash::Hash)]
    foo: (),
    #[ignore(Clone)]
    bar: (),
    #[ignore(std::hash::Hash, Clone)]
    baz: (),
    quux: ()
}
```

`std::hash::Hash` receives this `TokenStream`:

```rust
struct Foo {
    #[ignore]
    foo: (),
    bar: (),
    #[ignore]
    baz: (),
    quux: ()
}
```

Explanation:

- The `#[ignore]` applied to `foo` **contains** `std::hash::Hash`
- The `#[ignore]` applied to `bar` **does NOT contain** `std::hash::Hash`
- The `#[ignore]` applied to `baz` **contains** `std::hash::Hash`
- There is no `#[ignore]` applied to `quux`

The `#[ignore]` attribute is included for `foo` and `baz` in `std::hash::Hash`'s input `TokenStream`

Then it's up to the `std::hash::Hash` macro on how exactly it wants to use the `#[ignore]` attribute.

- In the common case, it will exclude `foo` and `baz` from the generated `std::hash::Hash` impl
- `std::hash::Hash` is allowed to ignore existence of the attribute.
- It could emit a `compile_error!("`#[ignore]` is not supported for this derive")`
  or, in the future, a [diagnostic](https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html)

## Standard library macros that support `#[ignore]`

The following standard library traits support `#[ignore]`:

- `PartialEq`
- `PartialOrd`
- `Ord`
- `Hash`
- `Debug`

## How this impacts code maintainability

Given a `Var` like this:

```rust
#[derive(Clone)]
pub struct Var<T> {
    pub ns: Symbol,
    pub sym: Symbol,
    meta: RefCell<protocols::IPersistentMap>,
    pub root: RefCell<Rc<Value>>,
    _phantom: PhantomData<T>
}
```

You want to implement:

- `PartialEq` and `Hash` such that only `ns` and `sym` fields are hashes and compared
- `Debug` such that it skips the `_phantom` field

### Without `#[ignore]` on fields

you'd need to implement those 3 traits manually:

```rust
#[derive(Clone)]
pub struct Var<T> {
    pub ns: Symbol,
    pub sym: Symbol,
    meta: RefCell<protocols::IPersistentMap>,
    pub root: RefCell<Rc<Value>>,
    _phantom: PhantomData<T>
}

impl<T> fmt::Debug for Var<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("ns", &self.ns)
            .field("sym", &self.sym)
            .field("meta", &self.meta)
            .field("root", &self.root)
            .finish()
    }
}

impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.ns == other.ns && self.sym == other.sym
    }
}

impl Hash for Var {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.ns, &self.sym).hash(state);
    }
}
```

Notes:

- It is logically incorrect for `Hash` and `PartialEq` implementations to differ, so you must remember to keep them in sync if `Var` changes
- You must remember to update the string names of the `Debug` if you ever rename the fields or `Var` itself

### With `#[ignore]`

```rust
#[derive(Clone, fmt::Debug, PartialEq, Hash)]
pub struct Var<T> {
    pub ns: Symbol,
    pub sym: Symbol,
    #[ignore(PartialEq, Hash)]
    meta: RefCell<protocols::IPersistentMap>,
    #[ignore(PartialEq, Hash)]
    pub root: RefCell<Rc<Value>>,
    #[ignore(PartialEq, Hash)]
    #[ignore(fmt::Debug)]
    _phantom: PhantomData<T>
}
```

Note: Multiple `#[ignore]` attributes can apply to the same field, which is the same as writing each argument to `ignore` in a single attribute.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `#[ignore]` attribute now *also* applies to:

- Fields of `struct`s
- Fields of enum variarnts
- Fields of `union`s

Notes:

- Fields can be either named or unnamed.
- When applied to fields, `#[ignore]` takes a list of [`SimplePath`](https://doc.rust-lang.org/reference/paths.html#simple-paths)s separated by comma,
  with an optional trailing comma, e.g. `#[ignore(Foo, Bar,)]`.
- The list is allowed to be empty: `#[ignore()]` but it **must** exist.
- Multiple `#[ignore]` attributes on a field are allowed to appear in a row, in which case their list of paths merges into a single `#[ignore]`.
  The derive macro still just receives a single `#[ignore]` if it's present in the list of paths, no matter how many `#[ignore]` attributes were used on the field.

```rust
#[derive(Foo)]
struct NamedFields {
    #[ignore(Foo)]
    ignored: ()
}

#[derive(Foo)]
struct UnnamedFields(#[ignore(Foo)] ());

#[derive(Foo)]
enum Enum {
    NamedFields {
      #[ignore(Foo)]
      ignored: ()
    },
    UnnamedFields(#[ignore(Foo)] ())
}

#[derive(Foo)]
union Union {
    #[ignore(Foo)]
    ignored: ()
}
```

### Name resolution

Paths supplied to `#[ignore]` must resolve to a derive macro applied to the current item.

```rust
use std::hash::Hash as RenamedHash;

#[derive(std::hash::Hash)]
struct Foo {
    #[ignore(RenamedHash)]
    ignored: ()
}
```

The above works, because `RenamedHash` *is* `std::hash::Hash`.

### Unknown derive

A path supplied to `ignore` that does not apply to the current derive, for example:

```rust
#[derive(Clone)]
struct Foo {
    #[ignore(PartialEq)]
    ignored: ()
}
```

Is a compile error.

### Duplicate derive

The list of paths passed to `ignore` is not allowed to contain more than 1 of the same path:

```rust
#[derive(Clone)]
struct Foo {
    #[ignore(Clone, ::core::clone::Clone)]
    ignored: ()
}
```

Both of the above generate compile errors.

## Inside the macro

The `#[ignore]` attribute(s) for a particular derive macro `Foo` applied to the current item will do one of 2 things
**once they become an input `TokenStream` to the derive macro `Foo`**:

- If the list of paths includes the derive macro `Foo` itself, then an `#[ignore]` attribute **without** any arguments is applied to the field
- If the list of paths to `ignore` does not name the the derive macro, then the `ignore` attribute is fully removed.

## Spans

Because input to the macro is different from what it is in the source code, we have to talk about
the `Span` that the derive macros see. Derives macros that don't support `#[ignore]` will want to report an error about it,
and they can do so by using span of the `ignore` identifier.

Given the following struct:

```rust
#[derive(PartialEq, Debug, std::hash::Hash)]
struct Foo {
  #[ignore(std::hash::Hash)]
  #[ignore(PartialEq, Debug)]
  ignored: ()
}
```

Each of `std::hash::Hash`, `PartialEq`, and `Debug` will receive the following input:

```rust
#[derive(PartialEq, Debug, std::hash::Hash)]
struct Foo {
  #[ignore]
  ignored: ()
}
```

The span of `#[ignore]` attribute received by derive macro `Debug` will be the same as the path `Debug` as *originally* passed to the
`#[ignore(Debug)]` attribute. Span is indicated with `^^^^` in the following examples:

- `std::hash::Hash`:

  ```rust
  #[derive(PartialEq, Debug, std::hash::Hash)]
  struct Foo {
    #[ignore(std::hash::Hash)]
             ^^^^^^^^^^^^^^^
    #[ignore(PartialEq, Debug)]
    ignored: ()
  }
  ```

- `PartialEq`:

  ```rust
  #[derive(PartialEq, Debug, std::hash::Hash)]
  struct Foo {
    #[ignore(std::hash::Hash)]
    #[ignore(PartialEq, Debug)]
             ^^^^^^^^^
    ignored: ()
  }
  ```

- `Debug`:

  ```rust
  #[derive(PartialEq, Debug, std::hash::Hash)]
  struct Foo {
    #[ignore(std::hash::Hash)]
    #[ignore(PartialEq, Debug)]
                        ^^^^^
    ignored: ()
  }
  ```

## New lints

This RFC additionally proposes to add 2 new deny-by-default lints:

- Types that implement `Eq` with fields that ignore **just one of** `Hash` or `PartialEq` issue a lint,
  because types `k1` and `k2` implementing `Eq` and `Hash` are expected to follow the property:

  ```
  k1 == k2 -> hash(k1) == hash(k2)
  ```

  Violating this property is a logic error, so it would be incorrect to `#[ignore]` only 1 of those traits.

- Types with fields that ignore **just one of** `PartialEq` or `PartialOrd` issue a lint,
  because it is logically incorrect for the implementations to differ.
  See the [documentation](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html) for details.

# Drawbacks
[drawbacks]: #drawbacks

It overloads `ignore` to mean 2 different things, as it currently has 1 meaning: functions marked with `#[test]` will be ignored from the test suite

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There was an [attempt](https://github.com/rust-lang/rust/pull/121053) to implement this feature with the `#[skip]` attribute before.
However, this lead to [unacceptable breaking changes](https://github.com/rust-lang/libs-team/issues/334#issuecomment-2183774372):

> To give an update (at long last), the crater report did confirm my suspicion. We have 4 confirmed root regressions: [1], [2], [3], [4], [4.1], [4.2].

Considering the breakage would compromise Rust's stabiligy guarantees, a different design is required:

> Given the crater results showing compatibility hazards, it sounds like this is going to need a design and lang RFC for an approach that avoids those. 

This is that RFC. It proposes a design that **avoids** any breaking changes, by re-using the conveniently named `ignore` attribute for a new purpose.

## What if someone already has a custom attribute macro named `ignore`?

Impossible.

The `#[skip]` built-in attribute could not be used because even the feature gate would break existing code from compiling.
In this RFC, the `#[ignore]` attribute avoids that.
This attribute is already built-in, and people cannot apply an attribute macro named `ignore` as that would be ambiguous:

```rust
use derive as ignore;

struct Foo {
    #[ignore]
    hello: ()
}
```

The above yields an ambiguity error:

```
error[E0659]: `ignore` is ambiguous
 --> src/main.rs:4:7
  |
4 |     #[ignore]
  |       ^^^^^^ ambiguous name
  |
  = note: ambiguous because of a name conflict with a builtin attribute
  = note: `ignore` could refer to a built-in attribute
note: `ignore` could also refer to the attribute macro imported here
 --> src/main.rs:1:5
  |
1 | use derive as ignore;
  |     ^^^^^^^^^^^^^^^^
  = help: use `crate::ignore` to refer to this attribute macro unambiguously
```

You **must** use the attribute as `#[ignore()]` where inside of the parentheses we can specify a list of paths.
Because today `#[ignore]` is already valid on fields:

```
warning: `#[ignore]` only has an effect on functions
 --> src/main.rs:2:5
  |
2 |     #[ignore]
  |     ^^^^^^^^^
  |
  = note: `#[warn(unused_attributes)]` on by default
```

It could be possible to implement `#[ignore]` on fields over an edition, maybe. This is discussed in the "Future Possibilities" section.

By explicitly **requiring** the list of arguments with parentheses, we can just feature-gate that syntax - which is currently invalid with a deny-by-default lint:

```
error: valid forms for the attribute are `#[ignore]` and `#[ignore = "reason"]`
 --> src/main.rs:2:5
  |
2 |     #[ignore()]
  |     ^^^^^^^^^^^
  |
  = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
  = note: for more information, see issue #57571 <https://github.com/rust-lang/rust/issues/57571>
  = note: `#[deny(ill_formed_attribute_input)]` on by default
```

Once this RFC is implemented, the above deny-by-default lint is promoted into a hard error on the stable channel.
This should be acceptable as it's unlikely someone uses this invalid syntax today - which doesn't do anything.
Even if so, the lint clearly states that it will become an error in a future release.
The lint was implemented in January 2019. See the [tracking issue](https://github.com/rust-lang/rust/issues/57571).
It has been almost 6 years since it became deny-by-default.
It should be fine to promote it into a hard error as that's what this RFC would require for the feature-gate

## Why not choose a new name

As seen with the `#[skip]` attribute attempt, it is likely to lead to breakages when we try to create a new built-in attribute for this.
The `ignore` keyword is very convenient for us, because it lets us implement this feature using an understandable keyword
while not compromising on the stability guarantees.

## How about `#[debug(ignore)]` or something like that?

That's how the ecosystem works at the moment. Crates such as `serde`, `clap` each have their own attributes for ignoring a field: `#[serde(skip)]`.

There is also the crate `derivative` which implements the ignore functionality for standard library traits:

```rust
#[derive(Derivative)]
#[derivative(Debug)]
struct Foo {
    foo: u8,
    #[derivative(Debug="ignore")]
    bar: String,
}
```

There are numerous disadvantages to not having a single attribute in the standard library for skipping fields when deriving:

- Blocks standard library derives like `Hash`, `Ord`, `PartialOrd`, `Eq`, `PartialEq` from ignoring fields, because
  if adding the 1 `skip` attribute lead to so much breakage imagine if we added 5 new attributes
- People have to learn each crate's own way of skipping fields, if it exists at all. With `#[ignore]` on fields, the user can instead use the same syntax
  for every derive macro. Derives that do not support `#[ignore]` attribute will be encouraged to emit a compile error to helpfully let the user know.
- Standardizes syntax and makes reading Rust code more predictable.
- Multiple `#[foo(ignore)]`, `#[bar(ignore)]` attributes each take up a separate line. A single `#[ignore(Foo, Bar)]` can instead nicely fit on a single line.
- **Compile times.** If we have a separate crate re-implement the standard library `Hash` derives and others, then the compile times will be much worse since
  they'll require additional dependencies like `syn` and miss out on the optimizations that can be gained with a derive macro that is
  included in the standard library

## What is the impact of not doing this?

- Less standardization in the ecosystem. Each crate can have its own way to do things.
- People are more likely to write buggy or incorrect code because they forget to update manual implementations of traits like `PartialEq`
  that only exist because they needed to exclude a field
- A lot of boilerplate that is usually taken care of by derive macros

# Prior art
[prior-art]: #prior-art

There are no other languages with something like derive macros that directly have this feature built-in

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None

# Future possibilities
[future-possibilities]: #future-possibilities

## `#[ignore]` without a list of paths is visible to all macros

It could be possible to allow just `#[ignore]` on fields without a list of paths -
which would make `#[ignore]` visible to every macro. For example, make this:

```rust
#[derive(Eq, std::hash::Hash, MyCustomDerive)]
struct Foo {
    foo: String,
    #[ignore]
    bar: u32
}
```

Equivalent to the following:

```rust
#[derive(Eq, std::hash::Hash, MyCustomDerive)]
struct Foo {
    foo: String,
    #[ignore(Eq, std::hash::Hash, MyCustomDerive)]
    bar: u32
}
```

If desired, such a change will be possible to make in the future, but it is not part of this RFC because the first code block **already compiles** (although with a warning - `ignore only has an effect on functions`).

## `#[ignore]` on enum variants

There is no known reason why this would be useful, but it could be added in the future if needed
