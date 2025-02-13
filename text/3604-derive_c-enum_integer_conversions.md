- Feature Name: derive_c-enum_integer_conversions
- Start Date: 2024-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

I want to define that a C-Style Enum is an enum that is defined like this:
```rust 
enum CStyleEnum {
  Variant1 = 10,
  Variant2 = 20,
  ...
  VariantX = some_integer
}
```
Since this is not documented by [the docs](https://doc.rust-lang.org/stable/std/keyword.enum.html) and [the book](https://doc.rust-lang.org/stable/book/ch06-01-defining-an-enum.html), but suported by the Rust Language, and is the topic of this RFC.

The code snippet below is basically what I want to propose 
```rust
//derive easier conversions for C-Style Enum
#[derive(TryFromInt, IntoInt)]
enum CStyleEnum {
  Variant1 = 10,
  Variant2 = 20,
  ...
  VariantX = some_integer
}

assert_eq!(CStyleEnum::try_from(10), Ok(CStyleEnum::Variant1)); //this works out of the box
assert_eq!((CStyleEnum::Variant1).into(), 10); // this works too
```

# Motivation
[motivation]: #motivation

**Why**:
- reduce boilerplate code, e.g. writing manual TryFrom and Into impls for the Integers your C-Style Enum will be constructed from, and will be converted to via Into (For use in generic code, where you might not be able to use ``as`` easily)
- Quality of life improvements, similar to [making deriving Default for enums possible](https://rust-lang.github.io/rfcs/3107-derive-default-enum.html)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- No new named concepts
- Explaining the feature 

We have a C-Style enum we derive TryFromInt and IntoInt for (name is subject to change). This allows us to save time writing boilerplate for constructing our enum from integers, and converting our enum to integers
```rust 
#[derive(TryFromInt, IntoInt)]
enum DNSOpCode {
  //each variant is just a number, that we asociate with 
  //one of these enum states/variants
  StandardQuery = 0,
  InverseQuery = 1
  ServerStatus = 2,
}
```
since the `DNSOpCode` enum maps each state to a number we can just use `as` to 
turn it into a number, the statement below is equal to zero, because the `DNSOpCode::StandardQuery` enum variant maps to 0 (defined by our enum)
```rust
let opcode_num = DNSOpCode::StandardQuery as u8; //this returns 0
```

likewise, we can also use the Into trait to do the same thing
opcode_num and opcode_num1 are the same, however opcode_num2 can be used in generic contexts where `as` cannot
```rust
let opcode_num2: u8 = DNSOpCode::StandardQuery.into(); //also returns 0
```
when going the other way, from a number to a DNSOpCode, the operation could fail, such as,
if our user wanted to convert the number 100 to a DNSOpCode, how could we possibly convert
that to an DNSOpCode, we can try,
```rust 
let try_get_opcode = DNSOpCode::try_from(100); // Err(())
```
well, since 100 is not one of the valid OpCodes defined in our enum, we couldn\`t convert
the number to a DNSOpCode, lets try with another number like 2

```rust
let try_again = DNSOpCode::try_from(2); // Ok(DNSOpCode::ServerStatus)
```
It worked! We can construct our C-style DNSOpCode enum **safely**, with **no boilerplate**, how cool

I think that this code all logically makes sense and there is really no other way I can think of to implement TryFrom<Integer> for a C-style enum, in terms of maintainability and readability I think the code isn\`t unambigous, and the reader can tell that there is one clear intent in the use of the try_form and into function, to try and get a `DNSOpCode` from an integer, or convert a `DNSOpCode` into an integer. Also, since this feature is completely additive, and it is opt-in, it won\`t break or deprecate any existing code. This feature isn\`t changing how anything new or old works, so I think it will be just as easy for beginners to learn as experienced rust programmers

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:


I want to propose a **TryFrom<{Integer}>** and **Into<{Integer}>** derive macro becomes available for all C-style enums. C-style enums are a subset of all integers, which is why C-style Enums can be cast to integers using the ``as {usize/u8/isize, etc..}`` syntax. I propose that we create a derive macro, that automatically allows C-Style Enums to be cast to Integers using ``.into()``, not just ``as``. Since only _some_ Integers can be turned into a C-Style Enum, I propose that a TryFrom<{Integer}> derive macro can be made for them, to allow easy and more generic conversion between a C-Style Enum and Integers.

## \#[derive(TryFromInt)] implementation
The logic of the derive macro would look something like this
```
make a TryFrom impl block for all of the integer types (u8,u16, etc..)
make an impl similar to this for all unsigned ints
impl TryFrom<u{8,16,..}> for InputtedEnum {
  type Error = (); //only one point of failure for conversion, so use unit
  fn try_from(value: u{8,16,...}) -> Result<InputtedEnum, ()>
    //this makes it so you can`t match a value for a field that has a value greater that u{8,16,...}::MAX
    //e.g. if an enum had a field with a value of 256, but we were implementing TryFrom for a u8, we would
    //have to do bounds checking and change the function body to only match on fields with a value less than
    //u{8,16,..}::MAX and more than u{8,16,...}::MIN
    let value = value as usize;
    match value { 
        InputtedEnum::Field1 as usize => Ok(InputtedEnum::Field1),
        InputtedEnum::FieldN as usize => Ok(InputtedEnum::FieldN),
        none_of_those_fields => Err(()) //no fields have inputted value, conversion failed
    }
  }
}
```
I\`m sure someone will be able to think of some cool optimizations for this, but this is plenty fast already

## \#[derive(IntoInt)] implmentaion
This one is really simple, its just using the `as` conversion inside the function, to convert the inputted enum to any number. But this allows it to be used inside generics where Into<{Integer}> is a trait bound, among others.
```
impl Into<{Integer}> for InputtedEnum {
  fn into(self) -> {Integer} {
    self as {Integer}
  }
}
```

# Drawbacks
[drawbacks]: #drawbacks

This feature is very insignificant, so thinking of any inherent, big drawbacks for it is a bit hard. It\`s main drawback is it isn\`t very important, might not be work the time of the rust team since C-style enums are not really used that much, perhaps due to their lack of documentation or that their usecases are pretty niche (making bitflags with unique names, etc.). I would be more than willing to write the derive macro and docs myself, since their isn\`t a lot of code to write. Maybe another drawback is that if not documented properly, developers might be confused why a TryFrom/Into implementation exists for their enum, since they never explicitly defined it (the derive macro is called TryFromInt, not TryFrom, etc..), which could cause a bit of confusion, but I think this can be avoided with good documentation

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?

I don\`t think any other designs have been introduced/considered since again, C-style enums are very niche, but this problem is not very logically hard, unlike `Futures`, `const evaluation`, etc.. The impact of not having these helper macros is that it makes a niche part of the language harder to use than it needs to be, and forces the developer to write more boilerplate to get their code to work as intended. This functionality could and is done by a library like [strum](https://docs.rs/strum/latest/strum/), however, since this is a very simple and niche feature, I still think its in the scope of being added to the language, also I think since everyone who has used rust extensively knows of the uses and implications of TryFrom and Into traits, instead of some other 3rd-party library defined traits (e.g. FromRepr in the case of strum).

# Prior art
[prior-art]: #prior-art

I didnt find any in the rfcs github, there probably aren\`t any

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Naming: Wether to name the derive macro TryFromInt or just TryFrom, with a doc comment about how its only for C-style enums and onyl converts into/from Integers, not anything else, I am currently leaning on TryFromInt and IntoInt right now.

# Future possibilities
[future-possibilities]: #future-possibilities


Extending C-Style enums to more types. I think it\`d be really cool if you could match things other than integers with C-style structs, e.g.
```rust 
enum ProgramFlags {
  Help = "--help",
  LogVerbose = "--verbose",
  LogToFile = "--file",
  ...etc
}
//in theory, C-style-enums could work for any type 
//that is PartialEq
enum PariatlEqVariants {
  Variant1 = [10, 20, 30, 40],
  Variant2 = [5, 10, 15, 20]
}

//or this 
enum UserInputError {
  InvalidNum = "please input a number between X and Y",
  TakenUsername = "A user with this username is already registered",
}

//then you could do something like this 
impl std::fmt::Display for UserInputError {
  //however, this might be a detriment to readability
  fn fmt(f, self) -> io::Result {
    writeln!(f, self as &str)? 
  }
}
```
to make C-Style structs less niche and usable for more usecases where you have an enum which stores no values, but represents a control flow for a program and/or a subset of inputs that are valid in the program, like in the example above. I think small changes like this one are a good first step to making C-Style enums easier to work with
