- Feature Name: Path Inference
- Start Date: 2023-06-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC introduces the leading-dot syntax for path inference in type construction. When the type is known from context, developers can write `.Variant`, `.Variant { field: 1 }`, and `.Variant(1)` for enums and `.{ field: 1 }` and `.(1)` for structs instead of writing out the type name.

## Motivation
[motivation]: #motivation

When working with enums in match statements or other contexts where variants are used repeatedly, developers commonly use glob imports (`use Enum::*`) or single-letter aliases (`use Enum as E`) to reduce verbosity. Glob imports risk name collisions, and single-letter aliases are disallowed in some codebases. Leading-dot syntax provides a standardized alternative that avoids both problems while maintaining clarity about where types come from.

```rust
// Current Approach (glob import)
use FooBar::*;
match my_enum {
    Foo => ...,
    Bar => ...,
}

// Current Approach (single letter import)
use FooBar as F;
match my_enum {
    F::Foo => ...,
    F::Bar => ...,
}

// Proposed Approach
match my_enum {
    .Foo => ...,
    .Bar => ...,
}
```

Function calls with struct parameters also benefit from this syntax. Named parameters in functions have been proposed multiple times for Rust. Leading-dot syntax for structs achieves a similar goal. Combining it with default field values only makes it more comparable. 

```rust
fn my_function(my_struct: MyStruct) { ... }

// Current Approach
my_function(MyStruct { field: 1 });

// Proposed Approach
my_function(.{ field: 1 });
```

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When the compiler knows what type to expect, instead of writing the full type, it's possible to write the type using the leading-dot syntax.

### All forms

- **Enum Variant (unit):** `.Variant`
- **Enum Variant (tuple):** `.Variant(1)`
- **Enum Variant (named fields):** `.Variant { value: 1 }`
- **Struct with (named fields):** `.{ value: 1 }`
- **Struct with (tuple):** `.(1)`

### Enum Variants

```rust
enum Status {
    Pending(f64),
    Complete { data: Vec<u8> },
    Failed 
}

fn update_status(s: Status) { /* ... */ }

// Current Approach
fn get_default_status() -> Status {
    Status::Pending(0.0)
}
update_status(Status::Pending(0.5));
update_status(Status::Complete { data: vec![1, 2, 3] });
update_status(Status::Failed);

// Proposed Approach
fn get_default_status() -> Status {
    .Pending(0.0)
}
update_status(.Pending(0.5));
update_status(.Complete { data: vec![1, 2, 3] });
update_status(.Failed);
```

### Match Arms

```rust
enum Status {
    Pending(f64),
    Complete { data: Vec<u8> },
    Failed 
}

// Current Approach
match status {
    Status::Pending(progress) => ...,
    Status::Complete { data } => ...,
    Status::Failed => ...,
}

// Proposed Approach
match status {
    .Pending(progress) => ...,
    .Complete { data } => ...,
    .Failed => ...,
}
```

### Struct Construction

```rust
struct Location(f64, f64);
struct WeatherData {
    location: Location,
    humidity: f64,
}

fn print_weather_data(weather_data: WeatherData) { /* ... */ }

// Current Approach
fn get_default_weather_data() -> WeatherData {
    WeatherData {
        location: Location(0.0, 0.0),
        humidity: 0.5,
    }
}
print_weather_data(WeatherData {
    location: Location(0.0, 0.0),
    humidity: 0.5,
});

// Proposed Approach
fn get_default_weather_data() -> WeatherData {
    .{
        location: .(0.0, 0.0),
        humidity: 0.5,
    }
}
print_weather_data(.{
    location: .(0.0, 0.0),
    humidity: 0.5,
});
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Syntax Additions

```rust
InferredPath :
    '.' VariantIdent
    '.' VariantIdent '(' ExprList? ')'
    '.' VariantIdent '{' FieldList? '}'
    '.' '(' ExprList? ')'
    '.' '{' FieldList? '}'
```

### Type Resolution

Path inference resolves the inferred path based on the concrete type expected at an expression or pattern position. These shall include return type annotations, function parameters, variable type annotations, or parent expressions.

### Scoping and Privacy

Path inference respects Rust's normal privacy rules. If a type is private, it remains inaccessible using path inference; that is to say that the leading dot syntax does not grant any additional access to otherwise inaccessible types.

## Drawbacks
[drawbacks]: #drawbacks

### Ambiguity in Functions with Multiple Enum Parameters

When multiple parameters share variant names, the leading-dot syntax can be confusing and ambiguous. 
```rust
enum RadioState { Disabled, Enabled }
enum WifiConfig { Disabled, Reverse, Enabled }

fn configure_wireless(radio: RadioState, wifi: WifiConfig) { /* ... */ }

configure_wireless(.Disabled, .Enabled);
```

Without looking at the function signature, it's unclear which argument corresponds to which parameter. This is problematic during code review.

**Note:** Developers can use explicit types when they deem it to be important. They can also restructure APIs to use struct parameters where field names provide more context. The below example is much clearer.
```rust
struct WirelessConfig {
    radio: RadioState,
    wifi: WifiConfig,
}

configure_wireless(.{ radio: .Disabled, wifi: .Enabled });
```

### Hidden Type Information in Code Review

When reading diffs or reviewing code without an IDE, the inferred type is not immediately visible. The below example might be difficult to understand.
```rust
configure(.{
    primary: .{
        mode: .Active,
        fallback: .{ strategy: .Exponential }
    }
});
```

**Note:** Ideally, the API designer should use more explicit function and field names, but this is not always possible.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Dot `.` over underscore `_`

In Rust, the underscore `_` is already used as a placeholder for type inference in type parameters and type arguments. As such, reusing `_` for path inference was considered. In the end, however, it was decided that this solution was not ideal and that dot syntax would be a better fit.

The main issue with underscores is that an underscore can be part of an identifier. As such, it's not possible to write `_Variant` the same way as `.Variant`. It would have to be written as `_::Variant`. This syntax is much longer and resembles a partially known type rather than an inferred type. In addition, a developer might think that it is okay to write `std::_::Variant`.

Additionally, the underscore `_` already has many meanings, such as discarding an unused variable or acting as a placeholder. Adding on additional meanings to the underscore would reduce clarity rather than increase consistency.

Leading dot syntax clearly communicates that the entire path is being inferred from context, and it's shorter.

## Prior art
[prior-art]: #prior-art

### `Default::default()`

Rust already has a form of type inference for construction: `Default::default()`. When the type is known from context, it's possible to write:

```rust
fn get_config() -> Config {
    Default::default()
}

let settings: Settings = Default::default();
```

People have also previously implemented path inference in a macro using `Default::default()`.

```rust
macro_rules! s {
    ( $($i:ident : $e:expr),* ) => {
        {
            let mut temp = Default::default();
            if false {
                temp
            } else {
                $(
                    temp.$i = $e;
                )*
                temp
            }
        }
    }
}

#[derive(Debug, Default)]
struct Foo { x: i32, y: u32 }

fn takes_foo(x: Foo) { dbg!(x); }

takes_foo(s! { x: 123, y: 456 });
```

### Swift

Swift, the main inspiration for this RFC, has had leading-dot syntax since its initial release in 2014. It's widely used throughout Swift codebases:

```swift
enum Status {
    case pending(f64)
    case complete(Data)
}

struct Data {
    var foo: String
}

func make_data() -> Data {
    .init(foo: "bar")
}

func make_status() -> Status {
    .pending(0.5)
}
```

This is functionally equivalent to the following:

```swift
func make_data() -> Data {
    Data(foo: "bar")
}

func make_status() -> Status {
    Status.pending(0.5)
}
```

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Generic Arguments in Enum Variants

Rust permits generic arguments after the variant for enum variants.
```rust
enum Status<Progress, Data> {
    Pending(Progress),
    Complete { data: Data },
}

Status::Pending<f64, Foo>(0.0);
Status::Complete<f64, Foo> { data: Foo::default() };
```
With leading-dot syntax, the analogous forms would be:
```rust
.Pending<f64, Foo>(0.0);
.Complete<f64, Foo> { data: Foo::default() };
```
The intent of path inference is that the path should be fully inferred from context. Allowing generic arguments doesn't make much sense because the type was supposed to be inferred already. It also creates an asymmetry; structs do not have an equivalent position for variant generics. This might be super confusing. 

Overall, it is not clear whether supporting generics for enum variants in path inference provides meaningful ergonomic benefits.

## Future possibilities
[future-possibilities]: #future-possibilities

### Generic Arguments in Path Inference

A future syntax addition could allow generics to appear in inferred typebases, such as:

```rust
use std::fmt::{Display, Debug};

fn update_status<Ok: Display, Err: Debug>(s: Result<Ok, Err>) {
    /* ... */
}

update_status(.Ok::<u8, String>(42));
update_status(.Err::<u8, String>("oops"));

// or
update_status(.<u8, String>::Ok(42));
update_status(.<u8, String>::Err("oops"));

struct Foo<T>(T);
struct Bar<Data> {
    data: Data,
}

fn foo<T>(t: T) -> Foo<T> {
    Foo(t)
}
fn bar<Data>(data: Data) -> Bar<Data> {
    Bar { data }
}

foo(.<u8>(42));
bar(.<String>{ data: "hello".to_string() });
```

### Impls in Path Inference

Another feature could be allowing calls to associated functions. While an associated function does not have to return the value that is expected, it's possible to show a type mismatch error. This would allow developers to write:

```rust
let items: Vec<Item> = .with_capacity(10);
```
