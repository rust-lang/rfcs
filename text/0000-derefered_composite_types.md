- Feature Name: `derefered_composite_types`
- Start Date: 2023-04-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

The proposal of Dereferenced (or Derefered) Composite Types is a Rust unification of Referenced Types and Dereferenced Types.

Dereferenced Composite Types could help also to get rid of constructor boilerplate.

The symbol `*` before the type name is a marker of Dereferenced Composite Type. 

The new operator `=&` (and `=&&`, `=&&&`, ...) as a compound of borrow and assignment (`x = &2` same as `x =& 2`) is also required.


# Motivation
[motivation]: #motivation

Currently, Rust has 
- (1) dereferenced primitive types (like `i32`)
- (2) referenced primitive types (like `&i32`) 
- (3) referenced composite types (like `Box<i32>`)

But there are no any dereferenced composite types.

This is not universal. We wish to improve this. So we propose types like `*Box<i32>`.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently we could write:
```rust
let foo : u32 = 5;  // dereferenced primitive type ~ *Stack<u32> == u32
let baz : &u32 = &5;  // referenced primitive type ~ Stack<u32> == &*Stack<u32> == &u32
let bar : Box<u32> = Box::new(5);  // referenced composite type

let foo2 : u32 = 5 + foo;
let foo3 : u32 = 5 + *bar;
```

Vriable `bar` has unhidden type `Box<u32>`, unhidden constructor `Box::new()` and unhidden deref `*bar`.

But variable `foo` has hidden type `Stack<u32>`, hidden constructor `Stack::new()` and hidden deref `foo`.

So, I propose to add star-marker to type for variables, that uses hidden constructor and hidden deref.

It is possible to write dereferenced composite type with new syntax:
```rust
let bar2 : *Box<u32> = 5;  // dereferenced composite type, auto-casting ::new(5)
let foo3 : u32 = 5 + bar2; // no additional deref-casting is need in use of *Box<u32>
```

**Note**: variable with Dereferenced Type could be constructed if type is explicitly known!

Sure, to use composite dereferenced types, those types must implement 2 traits: `Deref` and new `Construct`
```rust
impl<T> Deref .... fn deref(&self)
impl<T> Construct .... fn construct(&self)
```

That allows also to get rid of constructor boilerplate
```rust
let foo : *String = "some string";  // we get rid of String::from("some string")
let bar : *Box<u32> = 5;    // we get rid of Box::new(5)
let foo : *Box<*String> = "some string";  // we get rid of Box::new(String::from("some string"))
```

The `&=` operator is already in use and it has meaning bitwise and assignment.

It is required to include a compound borrow and assignment operator `=&` (and `=&&`, `=&&&`, ...) for use with derefed composite types.

We could also use dereferenced types for referenced types with new operator (`x = &2` same as `x =& 2`):
```rust
let foo : String =& "some string";  // free transmute &*String to String
let bar : Box<u32> =& 5;    // free transmute &*Box<u32> to Box<u32>
let foo : Box<String> =&& "some string";  // free transmute &*Box<&*String> to Box<String>
```

We expect, that `Box<T> == &*Box<T>`.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


Actually primitive types
```rust
let a : u32 = 5;
let b : u32 = 4 + a;
```

Rust desugars into something like (pseudo-code)
```rust
// Stack<T> must implement  Construct<T> Trait
impl<T> Construct<T> for Stack<T> {
   fn construct(t : T) -> Stack<T> {
      Stack::new(t)
   }
}

let a : *Stack<u32> = Stack::constuct(5)::as_deref_type();
let b : *Stack<u32> = Stack::constuct(4 + a::deref())::as_deref_type();
```

which desugars further by `Construct<T>` Trait into
```rust
let a : *Stack<u32> = Stack::new(5) as *Stack<u32>;
let b : *Stack<u32> = Stack::new(4 + a::deref()) as *Stack<u32>;
```

So, by analogy
```rust
let a : *Box<u32> = 5;
let s : *String = "some string";
let z : *Box<*String> = "some string";
```

must desugars into
```rust
let a : *Box<u32> = Box::constuct(5)::as_deref_type();
let s : *String = String::construct("some string")::as_deref_type();
let z : *Box<*String> = Box::constuct( String::construct("some string")::as_deref_type() )::as_deref_type();
```

which desugars further by `Construct<T>` Trait into
```rust
let a : *Box<u32> = Box::new(5) as *Box<u32>;
let s : *String = String::from("some string") as *String;
let z : *Box<*String> = Box::new( String::from("some string") as *String ) as *Box<*String>;
```

# Drawbacks
[drawbacks]: #drawbacks

Rust has a hack of `&str` type, which technically is a `str == *Str` type in terms of Dereferenced Composite Types.

With this proposal we must admit additional type hack `&str == str`.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

This feature allows to use more universal General Types such as `*T`.


# Prior art
[prior-art]: #prior-art

None known.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None so far.
