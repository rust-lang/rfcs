- Feature Name: `const_self_fields`
- Start Date: 2025-11-26
- RFC PR: [rust-lang/rfcs#3888](https://github.com/rust-lang/rfcs/pull/3888)
- Rust Issue: [rust-lang/rust#3888](https://github.com/rust-lang/rust/issues/3888)

# Summary
[summary]: #summary

This RFC proposes per-type fields that can be accessed through a value or trait object using new `const self` and `const self ref` syntax:

```rust
impl Foo{
    const self METADATA_FIELD: i32 = 5;
    const self ref REF_METADATA_FIELD: i32 = 10;
}
trait Bar {
    const self METADATA_FIELD: i32;
    const self ref REF_METADATA_FIELD: i32;
}
```
This allows code like:
```rust
fn use_bar(bar: &dyn Bar) {
    let x: i32 = bar.METADATA_FIELD; // const self
    let y: &'static i32 = &bar.REF_METADATA_FIELD; // const self ref
}
fn use_foo(foo: &Foo) {
    let x: i32 = foo.METADATA_FIELD; // const self
    let y: &'static i32 = &foo.REF_METADATA_FIELD; // const self ref
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
For monomorphized code where `Self` is known, `MyType::VALUE` is an excellent fit. 

However: You cannot directly read an associated const through a `&dyn Foo`. There is no stable, efficient way to write `foo.VALUE` where `foo: &dyn Foo` and have that dynamically dispatch to the concrete implementation’s const value. 

The common workaround is a vtable method:
```rust
trait Foo {
    fn value(&self) -> i32;
}
```

This forces a dynamic function call, which is very slow compared to the `const self` and `const self ref` equivalent, and does not have as much compiler optimization potential.

When using a trait object, `const self` and `const self ref` store the bits directly inside the vtable, so accessing it is around as performant as accessing a field from a struct, which is of course, much more performant than a dynamic function call.

Imagine a hot loop walking over thousands of `&dyn Behavior` objects every frame to read a tiny “flag”. If that’s a virtual method, you pay a dynamic function call on every object. With `const self` and `const self ref`, you’re just doing a metadata load, so the per-object overhead is noticeably much smaller.



# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### What is const self?

`const self` introduces metadata fields: constants that belong to a type (or trait implementation) but are accessed through a `self` expression. 

Example:

```rust
struct Foo;

impl Foo {
    const self CONST_FIELD: u32 = 1;
}

fn write_header(h: &Foo) {
    // Reads a per-type constant through a value:
    assert_eq!(h.CONST_FIELD, 1);
    let value: u32 = h.CONST_FIELD;
}
```

A `const self` field's type can have interior mutability, because the compiler does not operate on the field directly by its reference, even if it is stored in a trait object's metadata.
It first copies the field, and does the operations on that copied value, similar to how `const` variables work in rust.
This makes using it with interior mutability sound.

When using references like shown below:

```rust
let value : &u32 = &h.CONST_FIELD;
```

This works similarly to how `const` variables work in Rust: it copies `CONST_FIELD`, then takes a reference to that copy. Unlike a normal `const` item though, the resulting reference does **not** have a `'static` lifetime; it has a temporary lifetime, as if you had written:

```rust
let tmp: u32 = h.CONST_FIELD; // copied
let value: &u32 = &tmp;
```
### What is const self ref


`const self ref` is similar to `const self`, however, working on `const self ref` fields means working directly with its shared reference (no `mut` access). 
The type of a `const self ref` field must not have any interior mutability to ensure soundness. In other words, the type of the field must implement `Freeze`. This is enforced by the compiler.

Example:

```rust
struct Foo;

impl Foo {
    const self ref REF_CONST_FIELD: u32 = 1;
}

fn write_header(h: &Foo) {
    // Reads a per-type constant through a value:
    assert_eq!(h.REF_CONST_FIELD, 1);
    let reference: &'static u32 = &h.REF_CONST_FIELD;
}
```
`const self ref` field's references have `'static` lifetimes. 

Note that unlike normal `static` variables, you cannot rely on the reference of a `const self ref` field to be the same reference of the same `const self ref` field of the same underlying type.


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

Naming conventions for `const self` and `const self ref` fields follow the same conventions as other `const` and `static` variables (e.g. `SCREAMING_SNAKE_CASE` as recommended by the Rust style guide); this RFC does not introduce any new naming rules.

To be more specific about which trait's `const self`/`const self ref`  field should be accessed, a new `instance.(some_path::Trait.NAME)` syntax can be used. 

`T::FIELD` would give a compile-time error when `FIELD` is a `const self ref` or `const self` field. These fields are only accessible through value syntax (`expr.FIELD`), not type paths.
### How should programmers think about it?

Programmers can think of `const self`/`const self ref` metadata fields as “const but per-type” constants that can be read through references and trait objects, and a replacement for patterns like:
```rust
trait Foo {
    fn version(&self) -> u32; // just returns a literal 
}
```
Where the data truly is constant and better modeled as a field in metadata. 

### Teaching differences: new vs existing Rust programmers

For new Rust programmers, `const self` and `const self ref` can be introduced after associated constants:
* Types can have constants: `Type::CONST`
* Sometimes you want those constants visible through trait objects; that’s where `const self` metadata fields come in.
* Sometimes you want to be able to directly reference those constants. Good for when it is too large; that's where `const self ref` metadata fields come in.
* You can access `self.CONST_FIELD` even if self is `&dyn Trait`, as long as the trait declares it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
### Restrictions

For `const self FOO: T = ..;`, we only ever operate on copies, so its type having interior mutability is fine.

For `const self ref FOO: T = ..;`, we get a `&'static T` directly from the metadata; to keep that sound we additionally require `T: Freeze` so that `&T` truly represents immutable data.

Both `const self` and `const self ref` field's type are required to be `Sized`, and must have a `'static` lifetime .

Assume we have:

```rust
struct Foo;
impl Foo{
    const self X: Type = value;
    const self ref Y: OtherType = value;
}
```

then it can be used like:

```rust

let variable = obj.X; //ok. Copies it
let variable2 : &_ = &obj.X; // ok, but what it actually does is copy it, and uses the reference of the copy. Reference lifetime is not 'static.


let variable3 = obj.Y; // ok if the type of 'Y' implements Copy
let variable4 : &'static _ = &obj.Y; // ok. Lifetime of reference is 'static, uses the reference directly
```


### Resolution Semantics


For a path expression `T::NAME` where `NAME` is a `const self` or `const self ref` field of type `T`, it would give a compiler error. 
This is because allowing `T::NAME` syntax would also mean that `dyn Trait::NAME` syntax should be valid, which shouldn't work, since the `dyn Trait` type does not have any information on the `const` value. 

`const self` and `const self ref` fields are not simply type-level constants; they are value-accessible metadata.

For an expression `expr.NAME` where `NAME` is declared as `const self NAME: Type` or `const self ref NAME: Type`:

* First, the compiler tries to resolve `NAME` as a normal struct field on the type of expr.
* If that fails, it tries to resolve `NAME` as a `const self`/`const self ref` field from:
  * inherent impls of the receiver type
  * If that fails, it then tries to resolve scoped traits implemented by the receiver type, using the same autoderef/autoref rules as method lookup.
* A struct cannot have a normal field and an inherent `const self`/`const self ref` field with the same name. 
* If multiple traits, both implemented by type `T` and are in scope, provide `const self` or `const self ref` fields with the same name and `expr.NAME` is used (where `expr` is an instance of type `T`), that is also an ambiguity error. The programmer must disambiguate using `expr.(Trait.NAME)`.

### Trait objects

For a trait object: `&dyn Trait`, where `Trait` defines:

```rust
trait Trait {
    fn do_something(&self);
    const self AGE: i32;
    const self LARGE_VALUE: LargeType;
}
```

We would have this VTable layout
```
[0] drop_in_place_fn_ptr
[1] size: usize
[2] align: usize
[3] do_something_fn_ptr
[4] AGE: i32 //stored inline
[5] LARGE_VALUE: LargeType //stored inline
```
This layout is conceptual; the precise placement of metadata in the vtable is left as an implementation detail, as long as the observable behavior (one metadata load per access) is preserved.
### Lifetimes

Taking a reference to a `const self ref` field always yields a `&'static T`. This is sound since `const self ref` types are required to implement `Freeze`, are required to be `'static`, and only provide a shared reference (you cannot get a mutable reference to it)
```rust
let p: &'static i32 = &bar.REF_METADATA_FIELD;
```
However, you get a potentially different `'static` reference every time you use the same `const self ref` field from the same type. This is because the storage for a `const self ref` field potentially lives in a trait object’s metadata, and different trait objects of the same underlying type do not necessarily share the same exact metadata.
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

### Why this design?
* Explicitly value-only access (expr.NAME) keeps the mental model simple, as it functions similarly to a field access

* If you have a trait object, you can read its per-impl metadata.

* If you just have a type, associated consts remain the right tool.

* By forbidding `T::NAME`, we avoid:
  * Confusion over `dyn Trait::NAME`.
  * Having to explain when a const is “type-level” vs “metadata-level” under the same syntax.
* A metadata load is cheaper and more predictable than a virtual method call. Especially important when touching many trait objects in tight loops.

### Why not a macro/library?
A library or macro cannot extend the vtable layout or teach the optimizer that certain values are metadata; it can only generate more methods or global lookup tables. `const self`/`const self ref` requires language and compiler support to achieve the desired ergonomics and performance.

### Alternatives
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

* Is there a better declaration syntax than `const self ref NAME : Type`/`const self NAME : Type`?
* Is `obj.METADATA_FIELD` syntax too conflicting with `obj.normal_field`?
* Is `obj.(Trait.METADATA_FIELD)` a good syntax for disambiguating?
# Future possibilities
[future-possibilities]: #future-possibilities

* Faster type matching than `dyn Any`: Since `dyn Any` does a virtual call to get the `TypeId`, using `const self ref` to store the `TypeId` would be a much more efficient way to downcast.
