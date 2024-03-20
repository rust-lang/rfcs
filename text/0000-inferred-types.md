- Feature Name: Inferred Types
- Start Date: 2023-06-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This RFC introduces a feature allowing the base type of enumerations and structures to be inferred in contexts where strict typing information already exists. Some examples of strict typing include match statements and function calls. The syntax is `_::EnumVariant` for enumerations and `_ { a: 1 }` for constructing structs.


# Motivation
[motivation]: #motivation

Rust's goals include a clean syntax. Because of that, features like macros were created, making it easier to not repeat yourself. Having to write a type path every time you want to do something can be very annoying, repetitive, and, not to mention, hard to read. This is a huge problem, especially in large projects with heavy dependencies on enumerations. Additionally, with large libraries, developers can expect to import many traits, structures, and enumerations. One way developers have solved this is by importing everything from specific modules, like [`windows-rs`](https://github.com/microsoft/windows-rs). This is problematic because, at a glance, it can not be determined where a module comes from. It can be said that developers need a low-compromise solution to solve the problem of large imports and hard-to-read code. The intent of this RFC is to create something that is developer-friendly yet still conforms to all of Rust's goals. Finally, when using specific rust crates, it can be annoying to have to add one package specifically for a type definition (like `chrono`). Now it can be prevented!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When creating a struct or enumeration, inferred types can simplify the type to just an underscore. It is important to note, however, that they do not allow any sort of interaction with implementations and they don't work when the type is not specific enough to be inferred, like type parameters. Below are some examples of when they do and don't work.

Function calls (structs):
```rust
struct MyStruct {
   value: usize
}

fn my_function(data: MyStruct) { /* ... */ }

// my_function(MyStruct {
//     value: 1
// });
my_function(_ {
   value: 1
});
```

Function returns (enum):
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

It is important to note that `_` only represents the type; if you have (for example) another enumeration that can be coerced into the type, you will need to specify it manually.

```rust
enum MyEnum {
    One,
    Two
}

enum Example {
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

fn my_function(data: MyEnum) { /* ... */ }


my_function(Example::Alpha.into()); // ✅


my_function(_::Alpha().into()); // ❌ 
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `_` token can be used to simplify writing the type name explicitly when the type of its containing expression is known. The elided type does not need to be imported into scope. The `_` token does not allow access to implementations and trait implementations of any sort.

Mentioned above, implementations of any sort are not allowed
```rust
#[derive(Default)]
struct MyStruct {
    test: String
}

impl MyStruct {
    fn new() -> Self {
        Self {
            test: "Hello!"
        }
    }
}

fn do_something(argument: MyStruct) {/* ... */}

do_something(_::default())
//           ^^^^^^^^^^^^ Cannot call implementations methods on infered types.
do_something(_::new())
//           ^^^^^^ Cannot call implementations methods on infered types.
```


Finally, here are some examples of non-strict typings that can not be allowed.
```rust
fn do_something<T>(argument: T) -> Example {/* ... */}

do_something(_ { test: "Hello" })
//           ^^^^^^^^^^^^^^^^^^^ Cannot infer type on generic type argument
```



# Drawbacks
[drawbacks]: #drawbacks

In the thread [[IDEA] Implied enum types](https://internals.rust-lang.org/t/idea-implied-enum-types/18349), many people had a few concerns about this feature. 

These RFCs could cause bugs. An example of this is if a function has two enumeration parameters that share common variant names. Because it’s implied, it would still compile with this bug, creating unintended behavior, whereas by specifying the type names, the compiler would have thrown an error.
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
```

Another issue with this is that the syntax `_::` could be mistaken for `::crate_name` meaning a crate path.

Additionaly, the `_` opperator could confuse new users because new users may try to use the `_` opperator to try to access implementation methods.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There have been many ideas for what this operator should be, including `::` and `.`. Despite that, the underscore is the best because it has already been used to infer lifetimes and generic types. Additionally, the underscore by itself can be used to construct a structure, creating a consistent experience. Maintainers should accept this proposal because it can simplify writing Rust code and prevent the large problem of repetition in switch statements. 


# Prior art
[prior-art]: #prior-art


Apple’s Swift has had enumeration inference since 2014 and is used in many Swift codebases. One thing people have noticed, though, is that it could be used for so much more! In creating a Rust implementation, the goal was to extend what Swift pioneered and make it more universal. That is why this RFC proposes to make the underscore a general operator that can be used outside the small use case of enumerations and allow it to be used in structs.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The implementation of this feature still requires a deep dive into how exactly the compiler should resolve the typings to produce the expected behavior, however, algorithms for finding paths for inferred types already exist.

# Future possibilities
[future-possibilities]: #future-possibilities

Maybe in the future, implementation methods calls could be allowed.