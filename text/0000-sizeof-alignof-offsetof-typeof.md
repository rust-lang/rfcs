- Start Date: 2015-01-17
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add `sizeof`, `alignof` and `offsetof` operators similar to the ones found in C/C++.  
Add `typeof` operator for finding out the type of an expression or a struct field.

# Motivation

- Currently it is not possible to use size / alignment of a type in const expressions.
- It is not possible to find out size / alignment of a struct field without creating an instance of that struct.
- The equivalent of C's `offsetof(struct, field)` is not supported at all.

These are very annoying shortcomings for a "systems" language, so let's fix them!

# Detailed design

This RFC proposes to add the following operators:

| Operator            | Evaluates to...  |
|---------------------|------------------|
|`sizeof(T)`          |The size of type T.|
|`alignof(T)`         |The minimal alignment of type T.|
|`offsetof(F in S)`   |Offset of field F in struct S. F may be either an identifier for regular structs or an integer index for tuples and tuple structs.|
|`typeof(F in S)`     |The type of field F in struct S. F may be either an identifier for regular structs or an integer index for tuples and tuple structs.|
|`typeof(E)`          |The type of expression E.|

The first three yield a value of type `usize` and may be used in any expression contexts.  The `typeof` yields a type and may be used anywhere a type is expected.  
The two may be combined, for example the equivalent of C's `sizeof(MyStruct::field1)` can be written as `sizeof(typeof(field1 in MyStruct))`, `sizeof(pMyStruct->field1)` as `sizeof(typeof(pMyStruct.field1))`, and so on.

A more formal grammar:
```
expr          : ... | sizeof_expr | alignof_expr | offsetof_expr
type          : ... | typeof | typeof_fld
sizeof_expr   : "sizeof" "(" type ")"
alignof_expr  : "alignof" "(" type ")"
offsetof_expr : "offsetof" "(" struct_field "in" struct_type ")"
typeof        : "typeof" "(" expr ")" 
typeof_fld    : "typeof" "(" struct_field "in" struct_type ")"
struct_field  : ident | integer
```

Examples:
```rust
struct MyStruct {
    pub field1: i32,
    pub field2: f64
}

sizeof(MyStruct) == 16;
alignof(MyStruct) == 8;
offsetof(field2 in MyStruct) == 8;
sizeof(typeof(field1 in MyStruct)) == 4;

let mystruct = MyStruct { field1:1, field2:2 };
sizeof(typeof(mystruct.field1)) == 4;

sizeof(typeof(1 + 2)) == 4;
sizeof(typeof(1 in (i32, f64))) == 8;
```

# Alternatives

- Beef up rustc's constant folder, so it can see though \<raw-pointer-deref\>-\<field-select\>-\<get-address-of\> combos.  It would then be possible to express `sizeof` and `offsetof` in terms of raw pointer operations.  For example: `macro_rules! offsetof(($f:ident in $t:ty) => (unsafe{ &(*(0 as *const $t)).$f as usize }));`  
Cons: The implementations of these operators via pointer arithmetic use dubious tricks like dereferencing a null pointer, and may become broken by future improvements in LLVM optimizations.  Also, this approach seems to be falling out of fashion even in C/C++ compilers (I guess for the same reason): `alignof`is now a standard operator in C++11; the `offsetof` hasn't been standartized yet, but both GCC and clang implement it as a custom built-in.

- Implement a limited form of compile-time function evaluation (CTFE) by hard-coding knowledge of intrinsics such as `size_of<T>()` and `align_of<T>()` into the constant folder.  
Cons:
  - There is no syntax in Rust for referring to a struct field, other than in assignment context, so the equivalent of C++'s `sizeof Struct::field` cannot be expressed in it.
  - For `offsetof` this is an even bigger problem, because its core functionality requires referring to the field, whose offset we want to determine.  We could represent it as a string, i.e. `offset_of<T>(field: &str)`, but then what do we do in non-const contexts?  If we allow arbitrary string parameters, we'd have to bring back runtime reflection.  If we don't, it would be the only function in the language that *requires* a literal parameter.

# Unresolved questions

- Should we allow usage of private struct fields with `offsetof` and `typeof` or only of the public ones?
- Can these operators be made to work in array length expressions (and eventually in numeric type parameters,- when these get implemented)?  The trouble is that array lengths need to be known in the type checking phase, however the physical data type layouts are known only to LLVM in trans phase.


## Syntax bikeshedding

The above syntax looks very similar to regular function calls.  Some alternatives:
- Lose the parentheses: `sizeof MyStruct; alignof f64; sizeof typeof field2 in MyStruct`.  Cons: precedence isn't obvious, so people are probably going to use parentheses anyways.
- Macro-like syntax: `sizeof!(MyStruct); alignof!(f64); sizeof!(typeof!(field2 in MyStruct))`.  
Pros: distinct from regular functions; "non-standard" syntax is par for the course in macros; it's immediately obvious that these are evaluated at compile time.  
Cons: these macros would be "un-expandable", because there'd be no syntax to expand them to.
- Something completely new?  `sizeof#(MyStruct); offsetof#(field2 in MyStruct)`? `sizeof<(MyStruct)>; offsetof<(MyStruct, field2)>`? ...
