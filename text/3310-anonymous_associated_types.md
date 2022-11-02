- Feature Name: (`anonymous_associated_types`)
- Start Date: (2022-11-1)
- RFC PR: [rust-lang/rfcs#3310](https://github.com/rust-lang/rfcs/pull/3310)

# Summary
[summary]: #summary

Allows new anonymous types to be created within trait `impl` blocks which are only accessible as an associated type. 

# Motivation
[motivation]: #motivation

Many procedural macros generate a type for use with a user defined type (e.g. Serde's `MyStructVisitor` for `MyStruct`) which are not intended to be interacted with by the end user. Currently, those types must be public and must share some level of namespacing with other identifiers. Even if hidden in a module, the module name itself may conflict with other typenames. Allowing associated types to be defined inside of a trait `impl` block would make conflicts impossible, and also make it clearer that a type is only meant to be used to interact with a type as a trait object.

For builder and struct difference procedural macros, it is very helpful to be able to encode all of the fields of a struct as an enum (`MyStruct` -> [ `MyStructEnum` / `MystructBuilderEnum` / ?]), but that enum is only intended to be used to interact with other generated code. That generated type currently must be public and set as an associated type, which crowds the namespace and leads to confusion when types appear in editor autocomplete prompts or rustc error messages. 

Instatiating a type which would only be accessible as `<MyStruct as StructDiff>::Fields`, for example, would make it much clearer where the type is coming from and why it exists, as well as keeping the namespace clear. No other significant differences in functionality are desirable, the type should behave as a normal type and share a scope with the containing trait. 

Current state: 
```rust
pub trait Builder {
    type Fields;

    fn save_fields(&self) -> Vec<Self::Fields>;
}

struct MyStruct{
    field1: usize
}

// This type must be defined separately and match the visibility of the Builder trait
#[derive(Serialize, Deserialize, ...)]
pub enum MyStructBuilderFields = enum { Field1(usize) };

impl Builder for MyStruct {
    type Fields = MyStructBuilderFields;

    fn save_fields(&self) -> Vec<<Self as Builder>::Fields> {
        vec![<Self as Builder>::Fields::Field1(self.field1)]
    }
}
```
Many crates which currently generate types per-derive could potentially use this feature:
- [serde](https://crates.io/crates/serde): Creates a `MyStructVisitor` for every type which uses `#[derive(Serialize)]`
- [diff-struct](https://crates.io/crates/diff-struct): `MyStruct` -> `MyStructDiff` for `#[derive(Diff)]`
- [structdiff](https://crates.io/crates/structdiff): `MyStruct` -> `__MyStructDiffEnum` for `#[derive(difference)]`
- [typed-builder](https://crates.io/crates/typed-builder): `MyStruct` -> `MyStructBuilder` for `#[derive(TypedBuilder)]`


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Create a new enum/struct/union and assign it to an associated type inside of a trait. 

```rust
trait Builder {
    type Fields;

    fn save_fields(&self) -> Vec<Self::Fields>;
}

struct MyStruct{
    field1: usize
}

impl Builder for MyStruct {
    #[derive(Serialize, Deserialize)]
    type Fields = enum { Field1(usize) };

    fn save_fields(&self) -> Vec<<Self as Builder>::Fields> {
        vec![<Self as Builder>::Fields::Field1(self.field1)]
    }
}
```
Using this feature means that a type with a throwaway name (such as `MyStructBuilderFields`) will not have to be instatiated and exposed.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature should allow these anonymous associated types to be used as a standalone struct/enum so long as both `MyStruct` and `Builder` are in scope. No other significant changes from existing struct/enum handling are desirable. The exact implementation requirements in `rustc` will depend on which syntax is selected for declaring these types.

# Drawbacks
[drawbacks]: #drawbacks

This syntax for defining types could be somewhat confusing when paired with GATs, as an anonymous associated type would then have to satisfy the relevant bounds and potentially require nested `impl` blocks. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale
This design allows library authors to hide internal abstractions without significantly changing the language. Most builder/diff crates use a generated type currently, and the generated types are messy and exposing them not add value for the end user. The proposed syntax also requires the fewest changes to the language between the identified possible alternatives.

## Alternatives
Other possible designs could include symbol name generation which is guaranteed to not collide, with errors/autocompletion hints which reference the type as an associated type rather than using the generated name. Encapsulated (non-associated) type definitions inside of `impl` blocks could also solve some subsets of this problem, although the actual values then could not be returned from public trait functions. 

Downsides to not doing this include continued namespace pollution and subpar ergonomics for builder/diff libraries.

An alternative syntax could look like: 
```rust
...
impl Builder for MyStruct {
    // Allow types defined with this syntax to satisfy the associate type bound
    #[derive(Serialize, Deserialize)]
    enum <Self as Builder>::Fields { Field1(usize) }
}
...
```

# Prior art
[prior-art]: #prior-art

This feature has been discussed previously with no major objections, and generally seemed to be desired:
- https://github.com/rust-lang/rfcs/pull/2300#issuecomment-361626195
- https://internals.rust-lang.org/t/pre-rfc-anonymous-associated-types/7477

Swift allows a similar pattern, although it requires a named variable inside the extension scope.
```swift
protocol Builder {
    associatedtype Fields
    
    func save_fields() -> [Fields]
}
 
struct MyStruct {
    let field1: Int
}

extension MyStruct: Builder {
    enum MyStructFields {
        case field1(Int)
    }
    typealias Fields = MyStructFields
    
    func save_fields() -> [Fields] {
        [Fields.field1(self.field1)]
    }
}
```


# Unresolved questions
[unresolved-questions]: #unresolved-questions

The final syntax for assignment and interaction with derive macros will need to be resolved. The proposed syntax seems workable, but several forms are possible. General declaration of non-associated types in `impl` blocks is not considered in scope of this RFC, only associated types that will share the visibility of the parent struct.

# Future possibilities
[future-possibilities]: #future-possibilities

Defining types inside of `impl` scopes generally allows for cleaner code that allows library abstractions to be hidden from the end-user. `impl` blocks inside of `impl` blocks are a clear follow-up possibility.