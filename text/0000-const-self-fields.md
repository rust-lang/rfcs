- Feature Name: `const_self_fields`
- Start Date: 2025-11-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes per-type fields that can be accessed through a value or trait object using a new `const self` syntax:

```rust
impl Foo{
    const self METADATA_FIELD: i32 = 5;
}
trait Bar {
    const self METADATA_FIELD: i32;
}
```
This allows code like:
```rust
fn use_bar(bar: &dyn Bar) {
    let x: i32 = bar.METADATA_FIELD;
    let r: &'static i32 = &bar.METADATA_FIELD;
}
fn use_foo(foo: &Foo) {
    let x: i32 = foo.METADATA_FIELD;
    let r: &'static i32 = &foo.METADATA_FIELD;
}
```
When combined with traits, enables object-safe, per-implementation constant data that can be read through `&dyn Trait` in a more efficient manner than a dynamic function call, by storing the constant in trait object metadata instead of as a vtable method.
# Motivation
[motivation]: #motivation
Today, Rust has associated constants on types and traits:
```rust
trait Foo {
    const VALUE: i32;
}

impl Foo for MyType {
    const VALUE: i32 = 5;
}
```
For monomorphized code where Self is known, `MyType::VALUE` is an excellent fit. 


However: You cannot directly read an associated const through a `&dyn Foo`. There is no stable, efficient way to write `foo.VALUE` where `foo: &dyn Foo` and have that dynamically dispatch to the concrete implementation’s const value. 

The common workaround is a vtable method:
```rust
trait Foo {
    fn value(&self) -> i32;
}
```

This forces a dynamic function call, which is very slow compared to the `const self` equivalent, and does not have as much compiler optimization potential.

When using a trait object, `const self` stores the bits directly inside the vtable, so accessing it is around as performant as accessing a field from a struct, which is of course, much more performant than a dynamic function call.

Imagine a hot loop walking over thousands of `&dyn Behavior` objects every frame to read a tiny “flag”. If that’s a virtual method, you pay a dynamic function call on every object. With `const self`, you’re just doing a metadata load, so the per-object overhead is noticeably much smaller.



# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## What is const self?

`const self`introduces metadata fields: constants that belong to a type (or trait implementation) but can be accessed through a `self` expression.

### Example with a concrete type:

```rust
struct Foo;

impl Foo {
    const self CONST_FIELD: u32 = 1;
}

fn write_header(h: &Foo) {
    // Reads a per-type constant through a value:
    assert_eq!(h.CONST_FIELD, 1);

    // It is a &'static reference
    let ptr: &'static u32 = &h.CONST_FIELD;
}
```

### Trait objects and metadata fields

The main power shows up with traits and trait objects:

```rust
trait Serializer {
    // Per-implementation metadata field:
    const self FORMAT_VERSION: u32;
}

struct JsonSerializer;
struct BinarySerializer;

impl Serializer for JsonSerializer {
    const self FORMAT_VERSION: u32 = 1;
}

impl Serializer for BinarySerializer {
    const self FORMAT_VERSION: u32 = 2;
}

fn write_header(writer: &mut dyn std::io::Write, s: &dyn Serializer) {
    // Dynamically picks the implementation’s FORMAT_VERSION
    writer.write_all(&[s.FORMAT_VERSION as u8]).unwrap();
}
```

Accessing `FORMAT_VERSION` on a trait object is intended to be as cheap as reading a field from a struct: no virtual call, just a read from the vtable metadata for that trait object.
It is much more efficient than having a `format_version(&self)`, trait method, which does a virtual call.

On a non trait object, accessing `FORMAT_VERSION` will be as efficient as accessing a `const` value.

Naming conventions for `const self` fields follow the same conventions as other `const`/associated constants (e.g. `SCREAMING_SNAKE_CASE` as recommended by the Rust style guide); this RFC does not introduce any new naming rules.

To be more specific about which trait's `const self` field should be accessed, a new `instance.(some_path::Trait.NAME)` syntax can be used. 

NOTE: `T::FIELD` would give a compile-time error when `FIELD` is declared as `const self FIELD: Type`; `const self` fields are only accessible through value syntax (`expr.FIELD`), not type paths.
## How should programmers think about it?

Programmers can think of `const self` metadata fields as “const but per-type” constants that can be read through references and trait objects, and a replacement for patterns like:
```rust
trait Foo {
    fn version(&self) -> u32; // just returns a literal 
}
```
Where the data truly is constant and better modeled as a field in metadata. 

### Teaching differences: new vs existing Rust programmers

For new Rust programmers, `const self` can be introduced after associated constants:
* Types can have constants: `Type::CONST`
* Sometimes you want those constants visible through trait objects; that’s where `const self` metadata fields come in.
* You can access `self.CONST_FIELD` even if self is `&dyn Trait`, as long as the trait declares it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
## Restrictions
`const self` declarations:
* Must follow the same const-evaluation rules as associated constants (i.e., const expression is evaluated at compile time).
* Are per concrete type (for inherent impls) or per (Trait, ConcreteType) pair for trait implementations.
* The type `T` of a `const self` field must be `Sized` and `'static`, since it is stored in static metadata and references to it have type `&'static T`.

## Resolution Semantics


For a path expression  `T::NAME` where `NAME` is a `const self` field on type T, it would give a compiler error. 
This is because allowing `T::NAME` syntax would also mean that `dyn Trait::NAME` syntax should be valid, which shouldn't work, since the `dyn Trait` type does not have any information on the `const` value. 

`const self` fields are not simply type-level constants; they are value-accessible metadata.

For an expression `expr.NAME` where `NAME` is declared as `const self NAME: Type`

* First, the compiler tries to resolve `NAME` as a normal struct field on the type of expr.
* If that fails, it tries to resolve `NAME` as a `const self` field from:
  * inherent impls of the receiver type
  * If that fails, it then tries to resolve scoped traits implemented by the receiver type, using the same autoderef/autoref rules as method lookup.
* If both a normal struct field and a const self field of the same name are visible, there would be an ambiguity error, which can be resolved by `expr.(Trait.NAME)` syntax.
* If multiple traits, both implemented by type `T`, provide `const self` fields with the same name and `expr.NAME` is used (where `expr` is an instance of type `T`), that is also an ambiguity error. The programmer must disambiguate using `expr.(Trait.NAME)`.

## Trait objects

For a trait object: `&dyn Trait`, where Trait defines:

```rust
trait Trait {
    fn do_something(&self);
    const self AGE: i32;
}
```

We would have this VTable layout
```
[0] drop_in_place_fn_ptr
[1] size: usize
[2] align: usize
[3] do_something_fn_ptr
[4] AGE: i32 //stored inline
```
This layout is conceptual; the precise placement of metadata in the vtable is left as an implementation detail, as long as the observable behavior (one metadata load per access) is preserved.
## Lifetimes

Taking a reference to a `const self` field always yields a `&'static T`, because the data lives in static metadata
```rust
let p: &'static i32 = &bar.METADATA_FIELD;
```

# Drawbacks
[drawbacks]: #drawbacks

1. Programmers must distinguish:
   * Fields (expr.field),
   * Associated consts (T::CONST),
   * And const fields (expr.METADATA).
2. Vtable layout grows to include inline metadata, which:
   *  Increases vtable size when heavily used.
   * Needs careful specification for any future stable trait-object ABI.
3. Dot syntax now covers both per-instance fields and per-type metadata; tools and docs will need to present these clearly to avoid confusion.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why this design?
* Explicitly value-only access (expr.NAME) keeps the mental model simple, as it functions similarly to a field access

* If you have a trait object, you can read its per-impl metadata.

* If you just have a type, associated consts remain the right tool.

* By forbidding `T::NAME`, we avoid:
  * Confusion over `dyn Trait::NAME`.
  * Having to explain when a const is “type-level” vs “metadata-level” under the same syntax.
* A metadata load is cheaper and more predictable than a virtual method call. Especially important when touching many trait objects in tight loops.

## Why not a macro/library?
A library or macro cannot extend the vtable layout or teach the optimizer that certain values are metadata; it can only generate more methods or global lookup tables. `const self` requires language and compiler support to achieve the desired ergonomics and performance.

## Alternatives
Keep using methods:
```rust
fn value(&self) -> u32; // remains the standard way.
```
Downsides:
* Conceptual mismatch (constant-as-method).
* Extra indirection and call overhead.

# Prior art
[prior-art]: #prior-art

As of the day this RFC was published, there is no mainstream language with a similar feature. The common workaround is having a virtual function return the literal, but that does not mean we should not strive for a more efficient method.

This RFC can be seen as:
* Making explicit a pattern that compiler and runtimes already rely on internally (metadata attached to vtables).
* Exposing it in a controlled, ergonomic way for user code.
# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is there a better declaration syntax than `const self : TYPE`?
* Is `obj.METADATA_FIELD` syntax too conflicting with `obj.normal_field`?
* Is `obj.(Trait.METADATA_FIELD)` a good syntax for disambiguating?
# Future possibilities
[future-possibilities]: #future-possibilities

* Faster type matching than `dyn Any`: Since `dyn Any` does a virtual call to get the `TypeId`, using `const self` to store the `TypeId` Would be a much more efficient way to downcast.
