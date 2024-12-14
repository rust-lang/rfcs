- Feature Name: Inferred Types
- Start Date: 2023-06-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC introduces type inference via the underscore (`_`) syntax. Developers will be able to use the underscore (`_`) operator instead of writing out type names (e.g., `MyStruct`) to infer and construct `enum`s and `struct`s in function calls, match arms, and other places where significant type information is available. With this RFC, the following syntax will be available: `_::EnumVariant` and `_ { struct_field: 1 }`.

# Motivation
[motivation]: #motivation

Rust’s goals include having a concise syntax. Features such as macros were created for this reason; developers shouldn’t have to repeat themselves. Like this, having to write types to match arms, functions, and locations where the type is obvious can be both frustrating, repetitive, hard to read, and time-consuming. Existing solutions such as importing everything or importing types with a single character don’t cut it, as importing everything makes it hard to determine where types come from, and renaming types makes it hard to read. This problem is prevalent in libraries such as [`actix-web`](https://github.com/actix/actix-web/blob/e189e4a3bf60edeff5b5259d4f60488d943eebec/actix-http/src/ws/codec.rs#L123), it's inconvenient to write and hard to read the same type over and over. Developers need a low-compromise solution that is both readable and concise. This is the goal of this RFC: developing an agreeable syntax.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When constructing a `struct` or `enum`, inferred types can lower the verbosity of the code; you don’t need to type the type name for everything. You can use the `_` operator instead, with `_::EnumVariant` for `enum`s and `_ { struct_field: 1 }` for `struct`s.

Note: calling methods on `impl`s and `impl` traits are not supported.

Function calls:
```rust
struct AppSettings {
    pub enable_foobar: bool
}

fn update_settings(data: AppSettings) { /* ... */ }

// update_settings(AppSettings {
//     enable_foobar: true
// });
update_settings(_ {
    enable_foobar: true
});
```

Function returns:
```rust
fn my_function() -> MyEnum {
    // MyEnum::MyVariant
    _::MyVariant
}
```

Match arms:
```rust
enum Example {
    One,
    Two
}

fn my_fn(my_enum: Example) -> String {
    match my_enum {
        _::One => "One!",
        _::Two => "Two!"
    }
}
```

Note: that `_` only represents the type inferred from the function. Other types will have to be manually specified.
```rust
enum Numbers {
    One,
    Two
}

enum Letters {
    Alpha,
    Bravo
}

impl Into<MyEnum> for Example {
    fn into(self) -> MyEnum {
        match self {
            _::Alpha => _::One,
            _::Bravo => _::Two
        }
    }
}

fn use_numbers(number: Numbers) { /* ... */ }

use_numbers(Example::Alpha.into());
//          ^^^^^^^^^^^^^^^^^^^^^ Valid code

use_numbers(_::Alpha.into());
//          ^^^^^^^^^^^^^^^^^ error[E0599]: no variant or associated item named `Alpha` found for enum `Numbers` in the current scope
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The underscore (`_`) syntax is designed to allow concise writing of rust code. Most of the time, it is just like when the type is written explicitly. The compiler will first look for the underscore token (`_`) followed by, optionally, type parameters (`::</* type & lifetime parameters */>`), then brackets (for `struct`s) or a variant (for `enum`s). Below are some variations of valid syntax.  
  
```rust  
_::<&'static str>::EnumVariant(&"Hello, rust")  
_::<&'static str> {  
    field: &"Hello, rust"  
}  
_::EnumVariant  
_ {  
    field: 1  
}  
```  
  
Unlike explicitly writing the type, the underscore (`_`) syntax does not support accessing `impl` and `impl` trait data. This is because return types from `impl` methods may be different from the expected type of the variable, match arm, function call, etc…
  
```rust  
#[derive(Default)]  
struct MyStruct {
    value: usize
}

let test: MyStruct = _::default();  
//                   ^^^^^^^^^^ (example) Cannot access impl methods on inferred types. Help: replace `_` with ` MyStruct`.  
```  

Below are the places where types can be inferred, along with the `struct`s and `enum`s used for the examples.

Type definitions:
```rust
use std::default::Default;

#[derive(Default)]
struct FerrisData {
    pub alive: bool,
    pub mood: FerrisMood
}

#[derive(Default)]
enum FerrisMood {
    #[default]
    Happy,
    Sad,
    Angry,
    Unknown(Option<u8>)
}
```


Inferable locations:
- Variable Definitions:
  ```rust
  let value: usize = 2;
  let data: FerrisMood = match value {
      0 => _::Happy,
      1 => _::Sad,
      2 => _::Angry,
      num => _::Unknown(Some(num))
  };
  let happy: FerrisMood = _::Happy;
  ```
- Match arms:
  ```rust
  let value: FerrisMood = _::Happy;
  let data = match value {
      _::Happy => 0,
      _::Sad => 1,
      _::Angry => 2,
      _::Unknown(option) => option.unwrap_or(3)
  };
  ```
- Function calls:
  ```rust
  fn say_mood(mood: FerrisMood) {}
  say_mood(_::Happy);
  ```
- Struct fields:
  ```rust
  let data = FerrisData {
        mood: _::Happy,
        ..FerrisData::default()
  };
  ```
- Enum variants:
  ```rust
  let data = FerrisMood::Unknown(_::Some(1));
  ```


# Drawbacks
[drawbacks]: #drawbacks

In the thread [[IDEA] Implied enum types](https://internals.rust-lang.org/t/idea-implied-enum-types/18349), many people had a few concerns about this feature.

This feature could cause bugs. An example of this is if a function has two enumeration parameters that share common variant names. Using implied types here would make it impossible to know which settings are being set to what (without looking at the function signature).

```rust
enum RadioState {
    Disabled,
    Enabled,
}

enum WifiConfig {
    Disabled,
    Reverse,
    Enabled,
}

fn configure_wireless(radio: RadioState, wifi: WifiConfig) { /* ... */ }

// Hard to understand without function signature
configure_wireless(_::Disabled, _::Enable);
```

Another issue with this is that the syntax `_::` could be mistaken for `::crate_name` meaning a crate path.

Additionally, the `_` operator could confuse new users because new users may try to use the `_` operator to try to access implementation methods.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There have been many ideas for what this operator should be, including `::` and `.`. Despite that, the underscore is the best because it has already been used to infer lifetimes and generic types. Additionally, the underscore by itself can be used to construct `struct`s, creating a consistent experience.

As mentioned in the motivation, this would make code more concise and easier to read by removing the need to repeat type names already obvious.

# Prior art

[prior-art]: #prior-art

Apple’s Swift has had enumeration inference since 2014 and is used in many Swift codebases. One thing people have noticed, though, is that it could be used for so much more! In creating a Rust implementation, the goal was to extend what Swift pioneered and make it more universal. That is why this RFC proposes to make the underscore a general operator that can be used outside the small use case of enumerations and allow it to be used in structs.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The implementation of this feature still requires a deep dive into how exactly the compiler should resolve the typings. It may require a rewrite of how types are resolved within the compiler.

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, implementation methods calls could be allowed in certain cases with certain rules.
