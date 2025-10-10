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
      #[ignore(PartialEq, Eq)]
      span: Span,
      /// Value of the identifier
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

## Variants

You can also apply `#[derive]` to enum variants, too:

```rust
#[derive(Serialize, Deserialize, Debug)]
enum Status {
    Active,
    Inactive,
    #[ignore(Serialize, Deserialize)]
    Unknown,
}
```

## From the perspective of a `derive` macro

When a derive macro such as `#[derive(std::hash::Hash)]` is applied to an item like a `struct`:

- Any `#[ignore]` attributes that mention the derive itself, in this case `std::hash::Hash`, will be a part of the `TokenStream`
  that the `derive` macro receives - **with the list of derives removed**. The derive macro has no idea what other derives ignore
  this field, it just knows that it should ignore it.

  **Example:** `std::hash::Hash` will see `#[ignore] field: ()` when the input contains `#[ignore(std::hash::Hash, Clone)] field: ()`

- If the `#[ignore]` attribute **does not** mention the derive, then the attribute is removed completely from the macro's input `TokenStream`.
  The derive macro doesn't know that other derives ignore this field.

  **Example:** `Clone` will see just `field: ()` when the input contains `#[ignore(std::hash::Hash)] field: ()`

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
- Enum variants

Notes:

- Fields can be either named or unnamed.
- When applied to fields, `#[ignore]` takes a list of [`SimplePath`](https://doc.rust-lang.org/reference/paths.html#simple-paths)s separated by comma,
  with an optional trailing comma, e.g. `#[ignore(Foo, Bar,)]`.
- The list is allowed to be empty: `#[ignore()]` but it **must** exist. Just `#[ignore]` without the `()` is not allowed.
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

#[derive(Foo, Bar)]
enum Enum {
    #[ignore(Bar)]
    NamedFields {
      #[ignore(Foo)]
      ignored: ()
    },
    #[ignore(Bar)]
    UnnamedFields(#[ignore(Foo)] ()),
    #[ignore(Bar)]
    Unit,
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

A path supplied to `ignore` that does not apply to the current derive is disallowed. For example:

```rust
#[derive(Clone)]
struct Foo {
    #[ignore(PartialEq)]
    ignored: ()
}
```

### Duplicate derive

The list of paths passed to `ignore` is not allowed to contain more than 1 of the same path:

```rust
#[derive(Clone)]
struct Foo {
    #[ignore(Clone, ::core::clone::Clone)]
    ignored: ()
}
```

## Inside the macro

The `#[ignore]` attribute(s) for a particular derive macro `Foo` applied to the current item will do one of 2 things
**once they become an input `TokenStream` to the derive macro `Foo`**:

- If the list of paths includes the derive macro `Foo` itself, then an `#[ignore]` attribute **without** any arguments is applied to the field
- If the list of paths to `ignore` does not name the the derive macro, then the `ignore` attribute is fully removed.

## Declaring support

Derive macros must explicitly declare that they support the `ignore` attribute:

```rust
#[proc_macro_derive(Foo, attributes(foo, ignore))]
pub fn derive_foo(input: TokenStream) -> TokenStream {
    // ...
}
```

If the derive macro doesn't support `ignore`, then any usage of `#[ignore]` on fields will yield an error.

`#[ignore]` without parentheses, and `#[ignore = "some string"]` will compile and do nothing (with a warn-by-default or deny-by-default lint),
even if no support for the `ignore` attribute is declared by the derive macro. This is discussed further in the "Rationale and alternatives" section.

## Spans

Because input to the macro is different from what it is in the source code, we have to talk about
the `Span` that the derive macros see. Derive macros can use this span for various purposes.

For example, since this RFC does not make an attempt to fit every possible use-case of ignoring fields, some derive macros
might require additional information about *how* to ignore the field. They can use this span to create an error and re-direct
users to their custom `#[foo(ignore(...))]` that might take arguments for example.

### The span

Given the following struct:

```rust
#[derive(PartialEq, Debug, std::hash::Hash)]
struct Foo {
  #[ignore(std::hash::Hash)]
  #[ignore(PartialEq, Debug)]
  ignored: ()
}
```

Each of `std::hash::Hash`, `PartialEq`, and `Debug` will receive the following input, except that span of the `#[ignore]` will differ:

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

- Types with fields that ignore **just one, or just two of** `PartialEq`, `PartialOrd` and `Ord` issue a lint,
  because it is logically incorrect for the implementations to differ.
  See the [documentation](https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html) for details.

## Standard library derives supporting the `ignore` attribute

- `PartialEq`
- `PartialOrd`
- `Ord`
- `Debug`
- `Hash`

### `#[derive(PartialEq)]` does not implement `StructuralPartialEq` if any fields are ignored

By default, `#[derive(PartialEq)]` automatically implements [`StructuralPartialEq`](https://doc.rust-lang.org/std/marker/trait.StructuralPartialEq.html),
and the invariant automatically upheld is the following:

> interpreting the value of the constant as a pattern is equivalent to calling PartialEq

Essentially, given any type `Foo` implementing `PartialEq`, both A and B must be identical:

```rust
#[derive(PartialEq)]
struct Foo {
    foo: u32,
    bar: bool
}
const FOO: Foo = Foo { foo: 10, bar: false };

// A
match foo {
    FOO => print!("ok"),
    _ => panic!()
}

// B
match foo {
    Foo { foo: 10, bar: false }  => print!("ok"),
    _ => panic!()
}
```

But if any field is `#[ignore(PartialEq)]`d, then the property would be violated:

```rust
#[derive(PartialEq)]
struct Foo {
    foo: u32,
    #[ignore(PartialEq)]
    bar: bool
}
const FOO: Foo = Foo { foo: 10, bar: false };

// Then this
match foo {
    FOO => print!("not ok"),
    _ => panic!()
}

// Is actually this:
match foo {
    Foo { foo: 10, bar /* doesn't matter */ } => print!("not ok"),
    _ => panic!()
}

// The above is NOT equivalent to this:
match foo {
    Foo { foo: 10, bar: false }  => print!("ok"),
    _ => panic!()
}
```

Hence any type deriving `PartialEq` with fields that are marked `#[ignore(PartialEq)]` will not implement `StructuralPartialEq` automatically

# Drawbacks
[drawbacks]: #drawbacks

It overloads `ignore` to mean 2 different things, as it currently has 1 meaning: functions marked with `#[test]` will be ignored from the test suite

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There was an [attempt](https://github.com/rust-lang/rust/pull/121053) to implement this feature with the `#[skip]` attribute before.
However, this lead to [unacceptable breaking changes](https://github.com/rust-lang/libs-team/issues/334#issuecomment-2183774372):

> To give an update (at long last), the crater report did confirm my suspicion. We have 4 confirmed root regressions: [1], [2], [3], [4], [4.1], [4.2].

Considering the breakage would compromise Rust's stability guarantees, a different design is required:

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

It could be possible to implement `#[ignore]` (without a list of paths) on fields over an edition, maybe. This is discussed in the "Future Possibilities" section.

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

## `#[ignore]` (without parentheses) on fields already compiles. Would this be a breaking change?

You can currently apply `#[ignore]` to fields at the moment in 2 ways, which leads to a warn-by-default `unused_attributes` lint:

- `#[ignore]` **without parentheses**
- `#[ignore = "some string"]`

For example:

```rust
struct Foo {
    #[ignore]
    ignored_1: String,
    #[ignore = "for some reason"]
    ignored_2: String,
}
```

The above gives warnings:

```
warning: `#[ignore]` only has an effect on functions
 --> src/main.rs:2:5
  |
2 |     #[ignore]
  |     ^^^^^^^^^
  |
  = note: `#[warn(unused_attributes)]` on by default

warning: `#[ignore]` only has an effect on functions
 --> src/main.rs:4:5
  |
4 |     #[ignore = "for some reason"]
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Both attributes do nothing, but are syntactically valid. Under this RFC, **they will continue to compile with warnings**,
probably with a changed error message to alert the user that they do absolutely nothing.

Upgrading these warnings into a deny-by-default future incompatibility lint is discussed in the "Future Possibilities" section.

## Why not another name?

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
- People have to learn each crate's own way of skipping fields, if it exists at all.
  With `#[ignore]` on fields, the user can instead use the same syntax for every derive macro.

  If they try adding a derive macro to the the list of paths in the `ignore` attribute, the user will be notified by the
  compiler if the derive macro doesn't support the `ignore` attribute - as it is explicitly opt-in in `attributes(ignore)`
- Multiple `#[foo(ignore)]`, `#[bar(ignore)]` attributes each take up a separate line. A single `#[ignore(Foo, Bar)]` can instead nicely fit on a single line.
- **Compile times.** If we have a separate crate re-implement the standard library `Hash` derives and others, then the compile times will be much worse since
  they'll require additional dependencies like `syn` and miss out on the optimizations that can be gained with a derive macro that is
  included in the standard library

Other benefits of `ignore` attribute include:

- Standardizes syntax and makes reading Rust code more predictable.
- Allows language servers like rust-analyzer to provide suggestions for the list of paths an `ignore` attribute would accept,
  making this functionality more discoverable
- Similar to how multiple `#[derive]`s are merged into a single `#[derive]` by rustfmt, multiple `#[ignore]`
  attributes could be merged into a single `#[ignore]` attribute

## What is the impact of not doing this?

- Less standardization in the ecosystem. Each crate can have its own way to do things.
- People are more likely to write buggy or incorrect code because they forget to update manual implementations of traits like `PartialEq`
  that only exist because they needed to exclude a field
- A lot of boilerplate that is usually taken care of by derive macros

# Prior art
[prior-art]: #prior-art

Several crates in the Rust ecosystem currently support similar functionality.

- [`#[serde(skip)]`](https://serde.rs/attr-skip-serializing.html)
- [`#[clap(skip)]`](https://docs.rs/clap/latest/clap/_derive/index.html#command-attributes)
- [`derive_more::Debug`](https://docs.rs/derive_more/latest/derive_more/derive.Debug.html)
  is a more customizable version of the `std::Debug` derive and allows skipping individual fields with `#[debug(skip)]`

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

If desired, such a change will be possible to make in the future, but it is not part of this RFC because
the first code block already compiles - it would be a breaking change.

It's also not clear whether we'd want `#[ignore]` to work this way at all, so let's leave it up to a future
RFC to decide if giving `#[ignore]` (without parentheses) on fields meaning would be worth it.

## Promote `#[ignore]` on fields (without parentheses) and `#[ignore = "reason"]` on fields into a deny-by-default lint

Whilst it's not currently clear whether we want `#[ignore]` (without parentheses) on fields to actually do something,
we could upgrade the 2 currently useless forms of `#[ignore]` (without parentheses) on fields and `#[ignore = "reason"]` on fields into a
deny-by-default future incompatibility lint - just to be safe.

This lint is not part of the RFC, and can be discussed separately.

## `ignore` with arguments

It's possible that derive macros might find it useful to know *how* they should be ignored.
We could allow passing arguments to paths in `ignore`:

```rust
#[derive(MyTrait)]
struct S {
    #[ignore(MyTrait(<args>))] // <args> is any token stream
    foo: Foo,
    #[ignore(MyTrait)]
    bar: Bar,
    #[ignore(MyTrait = <arg>)] // <arg> is any expression
    baz: Baz,
}
```

Which would give the following input to `MyTrait`:

```rust
struct S {
    #[ignore(<args>)]
    foo: Foo,
    #[ignore]
    bar: Bar,
    #[ignore = <arg>]
    baz: Baz,
}
```

This could be backward-compatible to add, so this is left for a future RFC to propose.

However it's worth noting that there are several disadvantages with this:

- It could encourage misuse of the `ignore` attribute, using the arguments as a sort of "default" value,
  when it would be clearer to use [default field values](https://github.com/rust-lang/rust/issues/132162)
- Overloads meaning of the attribute, it is not necessarily always about "ignoring" any more, rather adding a condition to serializiation
- Makes it harder to read which derives are ignored. It's more reasonable to have a flat list of all ignored derives,
  and if there is any metadata about ignoring, derives can use their own helper attributes for that. Prefer:

  ```rust
  #[derive(Deserialize, Debug, Serialize, Parser)]
  struct MyStruct {
      #[ignore(Debug, Deserialize, Parser)]
      #[serde(ignore_if(is_meaningless(name)))]
      #[clap(ignore_if(is_meaningless(name)))]
      name: String,
  }
  ```

  Over stuffing all information in a single attribute:

  ```rust
  #[derive(Deserialize, Debug, Serialize, Parser)]
  struct MyStruct {
      #[ignore(Deserialize(is_meaningless(name)), Debug, Parser(is_meaningless(name)))]
      name: String,
  }
  ```

- None of the derives in the standard library would use this feature, and few crates would find it useful where default field values don't solve the issue. We would be supporting a mostly niche use-case
