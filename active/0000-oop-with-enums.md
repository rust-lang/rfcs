- Start Date: 2014-03-17
- RFC PR #:
- Rust Issue #:

# Summary

This is an alternative plan for virtual structs/functions that implements virtual structs by making it possible to inherit enums to support class inheritance trees, and by automatically dispatching traits over them.

It was originally inspired by Niko Matsakis' comment at https://github.com/rust-lang/rfcs/pull/5#issuecomment-37576304, but the end result is unrelated.

This proposal is complete in the sense that Java OO code, excluding inter-crate inheritance and protected, can be automatically translated to this, resulting in identical memory layout and machine code (in theory, assuming identical implementation choices and compiler backend).

The limitation to intra-crate can easily be lifted if desired, although it's not clear if that would be a good idea.

# Motivation

Provide an alternative to adding virtual structs and virtual functions that provides the same ability to express Java-like OO, but keeps the good design of current Rust and extends the current Rust primitives instead of adding a parallel system.

# Alternatives

## virtual struct and virtual fns at https://github.com/rust-lang/rfcs/pull/5

Advantages:
- No new keywords
- Enums, which we need anyway, are extended instead of inventing a new kind of datatype
- Instantiable and overridable structures are separated into struct and enum, rather than having a virtual struct that is both instantiable and overridable
- Only intra-crate inheritance by default, so that it can be better optimized and doesn't try to reinvent traits (virtual structs can also be made inter-crate only, but it is less natural than with enums)
- Overridable methods are declared in traits and not in anonymous impls so that related sets of overridable methods can be separated from each other and have a name and documentation
- It is only possible to override abstract methods and not implemented methods, which makes it harder to create badly designed class hierarchies (but we have syntax sugar to relieve the drawbacks in the form of "impl Trait for Type1, Type2, Type3, ...")

Disadvantages:
- Converting Java-style OO code may result in more verbose code due to the inability to override implemented methods and the lack of anonymously implemented virtual functions; however, this should not be a problem for well-designed OO code

## "fat objects" at https://github.com/rust-lang/rfcs/pull/9

[The "fat object" RFC is essentially equivalent to what I originally on the mailing list, and I changed my mind to the enum-based solution in this RFC]

The fat objects in that RFC can be implemented in terms of the extended enums in this RFC, like this:
```
trait $Trait
{}

enum Fat$Trait
{
    FatObj$Trait<T: $Trait> {priv data: T}
}

impl $Trait for Fat$Trait as match;
impl<T: $Trait> $Trait for FatObj$Trait<T> use(self.data) {}
```

This uses the "generic parameter in enum variants", "impl as match" and "impl use" extensions.

In addition to this pattern, this proposal provides closed subtyping, by directly implementing the structs inheriting from the enum, which is faster and allows the struct to call virtual methods on its own subclasses (done as sanely as possible).

It also provides the unification with enums described in the next section.

## Compared to all of them

- Gives us datasort refinements for enums for free by unifying them with struct inheritance
- Gives us generic type parameters for enum variants for free by unifying them with struct inheritance with generic type parameters
- Gives us optimization of inter-crate enum match to vtable dispatch for free by unifying with virtual function dispatch
- Provides an optional concise syntax to represent whole class hierarchies as nested enums by unifying enum variants and struct inheritance

# Changes to data structures

## Structs and enums can inherit enums

Data structures form a tree: internal nodes are called "enums", while leaf nodes are called "structs".

Inheritance is only possible within a single crate by default, and the enum may be inherited outside it only if "..." is specified in the enum declaration.

New syntax for structs:
```
struct Struct<T, U> [: Parent]
{
    [field1 : Type1],
    [field2 : Type1]
}
```

If Parent is not specified, this syntax behaves like the current struct.

If Parent is specified, it must be an enum containing the "..." specification, and the struct inherits its fields and acts as a new case for the enum's type tag.

## Enum have fields, and enum variants are syntax sugar for structs and enums inheriting the enum

New syntax for enums:

```
enum Enum<T, U> [: Parent]
{
   [field1 : Type1],
   [field2 : Type2],

   [TuplelikeVariant<V, W>(Type1, Type2)],
   [StructVariant<V, W> {field1 : Type1, field2 : Type2}],
   [enum EnumVariant<V, W> { <body like body of top-level enum}],
   [...]
}
```

Enums are significantly extended.

First of all, if Parent is specified, then it must be an enum containing the "..." specification: in this case, the enum inherits its field, and enum variants behave as new cases for the root enum's type tag.

The fields in the enum are common for all variants/derived structs.

Specifying variants in the enum is now syntax sugar equivalent to having structs inherit from the enum (which means that variants are types and you can have impls for variants).

Specifying an "enum" variant is syntax sugar for an enum inheriting the enum containing it.

The "..." specification indicates that it is allowed to inherit the enum in the current crate and not just inside the enum itself, and thus that cases declared in the enum are not exhaustive.

## Enum variants with generic parameters

This syntax naturally allows "enum variants" (i.e. structs or enums inheriting enums) to be parameterized by generics (not inherited from the enum).

This is a non-trivial extension to enums, which potentially changes affected enums to be unsized types and forces the type tags to be a vtable pointer.

## ABI repr specification

The #[repr] attribute for enums is extended to support a string parameter different than "C", which indicates that the type tag in the enum should be a vtable pointer and specifies the vtable ABI.

Specifying #[repr("Rust")] results in a Rust-specific vtable layout (implementation details to be decided) while #[repr("C++")] causes the compiler to generate a vtable layout compatible with the C++ ABI specified on the command line to rustc or the default one (using the same options that clang accepts).

## Constructing and matching structs that inherit enums 

Struct-like structs that inherit enums are constructed, matched, and destructured by listing all fields in the struct and all ancestors.

For Tuple-like structs that have ancestors with fields, we add this new mixed construction syntax, which is also used for pattern matching:
```
let struct_value = Struct{a: A, b: B}(X, Y);
```

Note that enums themselves cannot be tuple-like and must have named fields.

## Rationale

This design effectively unifies structs and enums, but we preserve two keywords to highlight the fundamental difference between enum that are "abstract" and inheritable and structs that are "concrete" and non-inheritable.

There is intentionally no data type that is both constructible and inheritable, since that can be trivially implemented by using an empty struct inheriting an enum, which allows to distinguish between pointers to the struct and pointers to the enum, which could point to any concrete struct.

The "..." specification is added so that it is possible to know at a glance whether one is looking at all the enum cases, or whether he needs to hunt them around in the crate.

Enum variants with generic parameters are a nice feature, and are required to support "fat trait objects" as described in the "Alternatives" section.

ABI repr is required to interoperate with C++, COM and other object-oriented ABIs.

# Changes to traits and impls

## Trait still DO NOT inherit structs

Traits are unchanged: in particular they DO NOT inherit structs or enums.

## Impls and inheritance

It is an error to implement a trait on a struct or an enum if the trait is implemented on a parent enum other than by using the "impl as match" feature described below.

## Impl for multiple structs/enum

Impl is extended to this syntax
```
impl Trait for EnumOrStruct1, EnumOrStruct2, ...
```

All target types must have a common enum ancestor, and this syntax is equivalent to implementing all methods as anonymous methods on that ancestor, and then implementing the trait on each target type by calling the anonymous method on the common ancestor.

## Impl as match

The new syntax
```
impl Trait1, Trait2, ... for Enum1, Enum2, ... as match;
```

is added, and implements TraitI for EnumJ (for all values of I and J) by matching over all structs or enums directly inheriting EnumJ and redirecting trait methods to the corresponding methods.

It is an error if any of the derived structs or enums does not implement the trait in question.

Of course, this can and will often be implemented by using a vtable tag and virtual dispatch as discussed later.

## Impl use

The new syntax
```
impl Trait1, Trait2, ... for EnumOrStruct use(<expr>) {
    <manually implemented methods>
}
```
is added, and implements the traits for EnumOrStruct by calling corresponding methods on \<expr\> (which is inserted exactly like a macro expansion with expr parameter).

Methods that are manually implemented in the impl block are of course not derived automatically.

Typically \<expr\> will be ```self.field_name```, thus redirecting the trait to field_name.

## Match and virtual dispatch

This proposal defines abstraction using "impl as match" in terms of match.

However, match on enums can and should be implemented using virtual dispatch when applied to an enum in the same crate.

Note that this also means that we get optimization of handwritten matches to virtual dispatch where appropriate for free.

## Rationale

Forbidding to implement traits overridden on parent enums is the key design idea here, and vastly simplifies the semantics compared to languages like Java. In particular, we don't really need a notion of "virtual dispatch" at the language level and can just use enum matching, and there is no difference between directly calling the implementation in the subclass, or the "impl as match" code in the superclass, since the latter just calls the former.

Designs that need to add "exceptional" overrides to a base case can still be implemented by implementing the base case using the "impl for multiple structs/enums" extension and specifying all non-exceptional derived structs/enums or inserting a new enum that is inherited by all the non-exceptional derived enum/structs, and implementing the base case there.

It is still possible to call the default code from the exceptional cases by putting the code in a plain impl method.

"impl as match" is added rather than automatically generating the match for traits implemented by all derived structs/enums so that it is possible to enforce the requirement that all derived structs/enums implement a trait, since doing the match automatically would result in the impl disappearing when a new derived struct/enum not implementing the trait is added.

Traits inheriting structs are not added, and equivalent functionality is provided by the combination of enums having fields, structs/enums inheriting enums and "impl as match".

"impl use" can be used to bridge enums to traits, by allowing an enum variant to redirect all trait implementations to a field with generic type, as shown in the "Alternatives" section.

"impl use" is also useful to override some functionality of a class by wrapping it with a newtype and using "impl use" to redirect most traits to it, except some overridden functions.

# Inter-crate inheritance

Inter-crate inheritance could be trivially added, but it's not clear if it is a good idea, since traits should be generally sufficient for abstraction between crates.

If desired it can be supported with either a "pub ..." inside enums, or by using "extern enum" instead of "enum".

Generic parameters on structs inheriting enums already makes enums potentially unsized and requiring vtable type tags, so inter-crate inheritance is free as a byproduct of that.

# Converting Java/C# code to the language in this RFC

This RFC allows to implement an OO system that is fully equivalent to the one in Java/C# (except for inter-crate inheritance and the "protected" keyword), but with a syntax that hopefully results in higher-quality designs.

To prove this, this section shows how to mechanically translate a Java/C# class structure into the language proposed in this RFC.

As you can see, this method uses all the extensions in this RFC except "impl use" (which has been added for fat trait objects insteadf).

## Transformations on the Java/C# source

These transformations convert the source code to a simpler subset of Java/C# where each class is either abstract or non-overridable, all virtual functions are interface implementations, an interface is either not overridden or all its methods are overridden, there is no super keyword, and each class/struct that is overridable outside the crate has no overrides within the crate.

1. For each non-final/virtual method that doesn't implement an interface method, introduce an interface including the virtual function, and make the class implement it
2. For each class/struct that is non-abstract and non-final/non-sealed, make it abstract and add a new empty final subclass, and add to it constructors that redirect to the original class that is now abstract; change all constructor invocations to invoke the constructors on the new subclass
3. For each overridden interface implementation method, also override all other methods of the interface, adding simple redirection to super.method for the missing ones
4. For each overridden method calling via super.method, move its implementation to a new final method, redirect the original method to it, and replace super.method with invocations to that method
5. For each class/struct that you want to be overridable outside the crate, add a new subclass that reimplements all the interfaces of the superclass by redirecting to the superclass if the interface is implemented, and with a dummy implementation otherwise

## Converting data to Rust

1. For each abstract class, define an enum with all its fields, inheriting from the enum defined for its superclass if any, and with the "..." specification
2. For each non-abstract class, define a struct with all its fields, inheriting from the enum defined for its superclass if any
3. Optionally, convert to an equivalent terse version by putting enum subclasses into the enum

## Converting operations to Rust

1. For each interface, add a trait with equivalent methods
1. For each method not implementing an interface, add an anonymous impl with it
2. For each interface implementation where none of the implementing methods are overridden, add an impl Trait for Class for it
3. For each interface implementation where methods are overridden by subclasses, add an "impl Trait for Class as match" and add an impl Trait for LIST where LIST is computed by coloring all subclasses that override the interface in question and all their ancestors and then setting LIST to all nodes visited by a tree traversal starting from the class in question that only visits children of colored nodes
4. For each class/struct that you added in step 5 of the Java transformation, annotate its "enum" so that it supports inter-crate overriding and replace all its impl bodies with "impl as match"
5. Optionally, globally compress "impl as match" lines by taking advantage of the ability to specify multiple types and traits per line, ideally with the set of lines with the smallest amount of characters


## Example 

Java:

```
class Foo
{
    public final void a() {...}
    public void b() {...}
    public void c() {...}
    public void d() {...}
    public void e() {...}
}

class Bar : Foo
{
    public void b() {... super.b() ...}
    public void c() {... super.c() ...}
}

abstract class Mid : Foo
{
    public boolean flag;
}

class Baz : Mid
{
   public void b() {... super.b() ...}
   public void d() {... super.d() ...}
}

class BazSib : Mid
{}

class Other : Foo
{}

```

Rust after this RFC:
```
enum Foo
{
    Foo_,
    Bar,
    enum Mid
    {
        flag: bool,
        Baz,
        BazSib,
    },
    Other
}        

trait Foo_b
{
    fn b(&self);
}

trait Foo_c
{
    fn c(&self);
}

trait Foo_d
{
    fn d(&self);
}

trait Foo_e
{
    fn e(&self);
}



impl Foo
{
    pub fn a(&self) {...}
    pub fn foo_b(&self) {...}
    pub fn foo_c(&self) {...}
    pub fn foo_d(&self) {...}
}

impl Foo_b, Foo_c, Foo_d for Foo, Mid as match;

impl Foo_b for Foo_, BazSib, Other
{
    fn b(&self) {self.foo_b()}
}

impl Foo_c for Foo_, Mid, Other
{
    fn c(&self) {self.foo_c()}
}

impl Foo_d for Foo_, Bar, BazSib, Other
{
    fn d(&self) {self.foo_d()}
}

impl Foo_e for Foo
{
    fn e(&self) {...}
}



impl Foo_b for Bar
{
    fn b(&self) {... self.foo_b(); ...}
}

impl Foo_c for Bar
{
    fn c(&self) {... self.foo_c(); ...}
}



impl Foo_b for Baz
{
    fn b(&self) {... self.foo_b(); ...}
}

impl Foo_d for Baz
{
    fn d(&self) {... self.foo_d(); ...}
}

```

Datatypes expressed in alternative verbose form:

```
enum Foo
{...}

struct Foo_ : Foo
{}

struct Bar : Foo
{}

enum Mid : Foo
{
    flag: bool,
    ...
}

struct Baz : Mid
{}

struct BazSib : Mid
{}

struct Other : Foo
{}
```

# Unresolved issues

- Should we have inter-crate inheritance?
- Is allowing multiple traits and multiple types in impl really a good idea, or does it interfere with grepping, separation, etc.?
- Should we perhaps allow "impl as match" to be inherited by default, so adding an enum in the hierarchy doesn't require to add an impl as match for it?
- I think that allowing to override implemented traits is bad, confusing and non-Rusty (calling methods on &Superclass doesn't call the methods defined there, but the ones in a subclass, etc.), but it makes the code more verbose if there are lots of exceptions; should we perhaps consider this as something that can be changed?

