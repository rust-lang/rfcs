- Start Date:
- RFC PR #: 
- Rust Issue #: 

# Summary

Support performance (time and space) sensitive data structures (such as the DOM
or an AST) by enhancing enums. In particular, offer a dynamically sized version
of an enum, called a struct. This struct is a generalisation of current structs
and supports efficient single inheritance. Structs and enum variants are unified
(up to the keyword and static vs dynamic size distinction) which means struct
variants, tuple structs, and enum variants as types emerge naturally. We add the
option of virtual dispatch of methods called on reference-to-concrete-type-
objects (in addition to virtual dispatch for reference-to-trait-objects which
exists today).

These changes are mostly backwards compatible, see the staging section for more
details.

# Motivation

Supporting efficient, heterogeneous data structures such as the DOM or an AST
(e.g., in the Rust compiler). Precisely, we need a form of code reuse which
satisfies the following constraints:

* cheap field access from internal methods;
* cheap dynamic dispatch of methods;
* cheap downcasting;
* thin pointers;
* sharing of fields and methods between definitions;
* safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

# Detailed design

Syntactically, we unify structs and enums (but not their keywords, but see
'alternatives', below) and allow nesting. That means enums may have fields and
structs may have variants. Both may have nested data; the keyword (`struct` or
`enum`) is only required at the top level. Unnamed fields (tuple variants/tuple
structs) are only allowed in leaf data. All existing uses are preserved. Some
examples:

plain enum:

```
enum E1 {
    Variant1,
    Variant2(int)
}

let x: E1 = Variant1;
let y: E1 = Variant2(4);
```

plain struct:

```
struct S1 {
    f1: int,
    f2: E1
}

let s: S1 = S1 {f1: 5, f: y};
```

enum with fields:

```
enum E2 {
    f: int,
    Variant1,
    Variant2(int)
}

let x: E2 = Variant2(f: 34, 23);
```

**Open question:** should we use `()` or `{}` when instantiating items with a
mix of named and unnamed fields? Or allow either? Or forbid items having both
kinds of fields?

nested enum:

```
enum E3 {
    Variant1,
    Variant2(int),
    VariantNest {
        Variant4,
        Variant5
    }
}

let x: E3 = Variant1;
let y: E3 = Variant4;
```

nested struct:

```
struct S1 {
    f1: int,
    f2: E1,
    S2 {
        f3: int
    }
}

let s1 = S1 {f1: 5, f: y}
let s2 = S2 {f1: 5, f: y, f3: 5}
```

All names of variants may be used as types (that is, from the above examples,
`E3`, `Variant1`, `VariantNest`, `Variant5`, `S1`, `S2` may all be used as
types, amongst others). Fields in outer items are inherited by inner items
(e.g., `S2` objects have fields `f1` and `f2`). Field shadowing is not allowed.

All leaf variants may be instantiated. Non-leaf enums may not be instantiated
(e.g., you can't create an `E3` or `VariantNest` object), but they my be used in
pattern matching. By default, all structs can be instantiated. Structs,
including nested structs, but not leaf structs, may be marked `virtual`, which
means they cannot be instantiated. Put another way, enums are virtual by
default. E.g., `virtual struct S1 { ... S2 { ... } }` means `S1` cannot be
instantiated, but `S2` can. `virtual struct S1 { ... virtual S2 { ... } }` would
mean `S2` could not be instantiated, but would be illegal because it is a leaf
item. The `virtual` keyword can only be used at the top level or inside another
`virtual` struct.

**Open question:** is the above use of the `virtual` keyword a good idea? We
could use `abstract` instead (some people do not like `virtual` in general, and
this use is different from the use described below for methods). Alternatively,
we could allow instantiation of all structs (unless they have pure virtual
methods, see below) or only allow instantiation of leaf structs.

We allow logical nesting without lexical nesting by using `:`. In this case a
keyword (`struct` or `enum`) is required and must match the outer item. For
example, `struct S3 : S1 { ... }` adds another case to the `S1` defintion above
and objects of `S3` would inherit the fields `f1` and `f2`. Likewise, one could
write `enum Variant3 : E3;` to add a case to the defintion of `E3`. Such items
are only allowed in the same crate as the outer item. Why?

    1. Prevents people from abusing virtual structs to create an open-ended
    abstraction: traits are more suitable in such cases.
    2. Downcasting is more optimizable, becomes O(1) instead of O(n). This is a
    common complaint against C++ RTTI (as pointed out on the mailing list).
    3. Prevents the 'fragile base class' problem - since there are no derived
    structs outside the base struct's crate, changing the base class has only
    limited and known effects.

When pattern matching data types, you can use any names from any level of
nesting to cover all inner levels. E.g.,

```
fn foo(x: E3) {
    // All three versions give correct coverage
    match x {
        E3 => {}
    } 
    match x {
        Variant1 => {}
        Variant2(_) => {}
        VariantNest => {}
    } 
    match x {
        Variant1 => {}
        Variant2(_) => {}
        Variant4 => {}
        Variant5 => {}
    } 
}
```

The only difference between structs and enums is in their representation (which
affects how they can be used). enum objects are represented as they are today.
They have a tag and are the size of the largest variant plus the tag. A pointer
or reference to an enum object is a thin pointer to a regular enum
object. Nested variants should use a single tag and the 'largest variant' must
take into account nesting. Even if we know the static type restricts us to a
small object, we must assume it could be a larger variant. That allows for
trivial coercions from nested variants to outer variants. We could optimise this
later, perhaps.

Non-leaf struct values are unsized, that is they follow the rules for DSTs. A
programmer cannot use non-leaf structs as value types, only pointers to such
types may exist. E.g., (given the definition of `S1` above) one can write `x:
S2` (since it is a leaf struct), `x: &S2`, and `x: &S1`, but not `x: S1`. Struct
values have their minimal size (i.e., their size does not take into account
other variants). This is also current behaviour. Pointers to structs are DST
pointers, but are not fat. They point to a pointer to a vtable, followed by the
data in the struct. The vtable pointer allows identification of the concrete
struct variant.

Pointer-to-struct objects may only be dereferenced if the static type is a leaf
struct. Dereferencing gives only the concrete object with no indication of the
vtable.

The runtime representation of pointer-to-leaf struct objects is changed from the
current representation in that the pointer is a pointer to `[vtable_ptr, data]`
rather than a pointer to `data`. However, since dereferencing must give the data
(and not the `[vtable_ptr, data]` representation), this difference is only
observable if the programmer uses unsafe transmutes.

To summarise the important differences between enums and structs: enum objects
may be passed by value where an outer enum type is expected. Struct objects may
only be passed by reference (borrowed reference, or any kind of smart or built-
in pointer). enum values have the size of the largest variant plus a
discriminator (modulo optimisation). Struct values have their minimal (concrete)
size plus one word (for the vtable pointer). For example,

```
enum E {
    EVar1,
    EVar2,
    EVar3(i64, i64, i64, i64),
}

struct S {
    SVar1,
    SVar2,
    SVar3(i64, i64, i64, i64),
}

fn foo_e(e: E) {...}
//fn foo_s(s: S) {...}

fn foo_er(e: &E) {...}
fn foo_sr(s: &S) {...}

fn foo_e1(e: EVar1) {...}
fn foo_s1(s: SVar1) {...}
```

Here an instance of `EVar1` has size 40 bytes (assuming a 64 bit discriminator)
and an instance of SVar1 has zero size. An instance of `&E` has size 8 bytes
(assuming 64 bit pointers) and points to an object of size 40 bytes. An instance
of `&S` has size 8 bytes and pointers to an object of unknown size. If the
dynamic type is `&SVar1` then the pointed-to data has size 8 bytes (zero size
value plus a pointer to a vtable).

The function `foo_s` is a type error because `S` is an unsized type (DST). The
other functions are all valid.

A programmer would typically use enums for small, similar size objects where the
data is secondary and discrimination is primary. For example, the current
`Result` type. Structs would be used for large or diversely sized objects where
discrimination is secondary, that is they are often used in a polymorphic
setting. A good candidate would be the AST enum in libsyntax or the DOM in
Servo.

Matching struct objects (that is pointer-to-structs) should take into account the
dynamic type given by the vtable pointer and thus allow for safe and efficient
downcasting.

## Methods

Methods may be marked as `virtual` which allows them to be overridden by a sub-
struct. Overriding methods must be marked `override`. It is an error for a
method to override without the annotation, or for an annotated method not to
override a super-struct method. Methods may be marked as both `override` and
`virtual` to indicate that they override and may in turn be overridden. (Methods
marked `override` but not `virtual` must override but may not be overriden). A
method without the `virtual` annotation is final (in the Java sense) and may not
be overridden. Virtual methods may be given without a body, these are pure
virtual in C++ terms. This is only allowed if the struct is also marked
`virtual` (so that the struct cannot be instantiated). Non virtual methods will
be statically dispatched, as they are currently. Virtual methods are dispatched
dynamically using an object's vtable.

**Open question:** alternative to `virtual` keyword - `dynamic`.

## Subtyping and coercion

Nothing in this RFC introduces subtyping.

Inner enum values can implicitly coerce to outer enum values as can enum pointer
values.

Inner struct pointer values can implicitly coerce to outer struct pointer
values. Note that there is no coercion between struct values. Since all but leaf
structs are unsized, they may not be dereferenced. Thus we are immune to the
object slicing problem from C++. Mutable references cannot be upcast (coerced)
in this way since it would be unsafe (once the scope of the 'super-type' borrow
expires, the 'sub-type' reference can be accessed again and the pointee may have
changed type). Coercion of mutable `Box` pointers is allowed.

**Open question: We could choose to force explicit coercions. It would make
sense for the behaviour to match sub-traits, whatever we decide for that.

Via the DST rules, it should fall out that these coercions work for smart
pointers as well as `&` and `Box` pointers.

If in the future we decide that subtyping is useful, we could add it backwards
compatibly.

## Generics

(I feel the syntax could be nicer here, any ideas?)

Nested structs must specify formal and actual type parameters. The outer items'
type parameters must be given between `<>` after a `:` (similar to the
inheritance notation, but no need to name the outer item). E.g.,

```
struct Sg<X, Y> {
    Sgn<X, Y, Z> : <X, Y> {
        field: Foo<X, Z>
    }
}

let x = Sgn<int, int, int> { field: ... };
```

In the nested notation only, if an item has exactly the same type parameters as
its parent, they may be ommitted. For example, for

```
struct Sg<X, Y> {
    Sgn2<X, Y> : <X, Y> {
        field: Foo<X>
    }
}

let x = Sgn2<int, int> { field: ... };
```

the programmer may write

```
struct Sg<X, Y> {
    Sgn2 {
        field: Foo<X>
    }
}

let x = Sgn2<int, int> { field: ... };
```

When non-nested syntax is used, all type parameters must be specified, including
actual type parameters for the parent. (Note also that the super-type is named
whether or not type parameters are present). E.g.,

```
struct Sg<X, Y> {}

struct Sgn<X, Y, Z> : Sg<X, Y> {
    field: Foo<X, Z>
}

let x = Sgn<int, int, int> { field: ... };
```

For enums, only the outermost enum may have type parameters. All nested enums
implicitly inherit all those type parameters. This is necessary both for
backwards compatibilty and to know the size of any enum instance.

## Privacy

The privacy rules for fields remain unchanged. Nested items inherit their
privacy from their parent, i.e., module private by default unless the parent is
marked `pub`.

**Open question:** is there a use case for allowing nested items to be marked
`pub`? That is having a private parent but public child. What about the
opposite?


## Drop

Traits may be marked `inherit` (alternatively, we could use `virtual` here too,
although this is probably overloading one keyword too far): `inherit Trait Tr
{...}`. This implies that for an item `T` to implement `Tr` any outer item of
`T` must also implement `Tr` (possibly providing a pure virtual declaration if
the outer item is itself virtual). This is checked where the impl is declared,
so it would be possible that an impl could be declared for an outer item in a
different module but due to the visibility rules, it is invisible, this should
be a compile error. Since `impl`s are not imported, only traits, I believe this
means that if a trait is marked `inherit`, then anywhere an implementation for
an inner item is visible, then an implementation for the outer item is also
visible.

`Drop` is marked `inherit`.

Where an object goes out of scope, the compiler will check for the Drop trait
like it does today. However, if it finds one on the static type, then it will
generate code which calls all implementations of drop up the inheritance
hierarchy (rather than calling a single destructor). Note that by marking the
`Drop` trait as `inherit`, it is not possible that the dynamic type has a
destructor, but the static type does not.

I believe this is possible by walking the vtable and calling all methods rather
than just the first. So this should not require any additional reflection
capabilty.

I believe that this gives the desired behaviour and is backwards compatible,
other than the addition of the `inherit` keywords. It is the desired behviour
for destructors, but it is a little bizarre when thought of in terms of regular
virtual method calls. I think this is the least worst option, however.

A possible generalisation of this is a mechanism for requiring inner items
to implement a trait (with or without implementing it in the outer item, the
former case is like saying "must override")? This is kind of dual to the idea
above that if an outer item implements a trait, then the inner trait appears to
implement it too, via coercion. (ht Niko).


## Calling overridden methods

If a method is overridden, we should still be able to call it. C++ uses `::`
syntax to allow this, UFCS should let us do this. Since all such uses would use
static dispatch, we would use self-as-argument syntax, e.g.,
`BaseType::method(self, ...)`.


## Trait matching

When searching for traits to match an expression, super-structs/enums should
also checked for impls. Searching for impl candidates is essentially a matter of
dereferencing a bunch of times and then trying to apply a subset of coercions
(auto-slicing, etc.), and then auto-borrowing. With this RFC, we would add
checking of outer items to the set of coercions checked. We would only consider
these candidates for structs if the type of `self` is a reference type.


# JDM's example

From https://gist.github.com/jdm/9900569

```
virtual struct Node {
    parent: Rc<Node>,
    first_child: Rc<Node>,
}

struct TextNode : Node {
}

virtual struct Element : Node {
    attrs: HashMap<str, str>
}

impl Element {
    fn set_attribute(&mut self, key: &str, value: &str)
    {
        self.before_set_attr(key, value);
        //...update attrs...
        self.after_set_attr(key, value);
    }

    virtual fn before_set_attr(&mut self, key: &str, value: &str);
    virtual fn after_set_attr(&mut self, key: &str, value: &str);
}

struct HTMLImageElement : Element {
}

impl HTMLImageElement {
    override fn before_set_attr(&mut self, key: &str, value: &str)
    {
        if (key == "src") {
            //..remove cached image with url |value|...
        }
        Element::before_set_attr(self, key, value);
    }    
}

struct HTMLVideoElement : Element {
    cross_origin: bool
}

impl HTMLVideoElement {
    override fn after_set_attr(&mut self, key: &str, value: &str)    
    {
        if (key == "crossOrigin") {
            self.cross_origin = value == "true";
        }
        Element::after_set_attr(self, key, value);
    }
}

fn process_any_element(element: &Element) {
    // ...
}

fn foo() {
    let videoElement: Rc<HTMLVideoElement> = ...;
    process_any_element(videoElement);

    let node = videoElement.first_child;

    // Downcasting
    match node {
        element @ Rc(Element{..}) => { ... }
        _ => { ... }
    }
}
```

# Drawbacks

We are adding a fair bit of complexity here, in particular in allowing nesting
of structs/enums. The reduction in complexity by unifying structs and enums has
clearer advantages to the language implementation than to users of the language.

The difference between a struct and enum is subtle, and probably hard to get
across in a tutorial. On the other hand, the two are satisfying clearly
different use cases with different priorities. I believe the extra complexity
does not need to be paid for by every user in the sense that, unless you
specifically want to use these features, you don't need to know about them.

# Alternatives

There have been many proposals for alternative designs and variations on this
design. One minor variation would be to use anonymous fields rather than `:`
extension for struct inheritance. An alternative proposal is to allow traits to
extend a single struct and add subtyping appropriately. We would then need to
add support for some kind of RTTI (possibly using a trait and macros) to allow
safe and efficient downcasting.

## Some other RFCs

* [Virtual Structs (5)](https://github.com/rust-lang/rfcs/pull/5) Stays as
  closely as possible to single inheritance in Java or C++. Touches only
  structs so does not unify structs and enums. That means we end up with two
  design choicesl (enums or virtual structs), where there probably shouldn't be.
  The scheme for defining virtual methods is used in this RFC.

* [Fat objects (9)](https://github.com/rust-lang/rfcs/pull/9) Proposes using a
  pointer to a vtable+data and treating it as DST for representing objects. A
  very similar scheme is used in this RFC. RFC 9 does not actually propose a
  mechanism for supporting inheritance and efficient virtual methods, just a
  representation for objects (it suggests using Niko's earlier
  [proposal](http://smallcultfollowing.com/babysteps/blog/2013/10/24/single-inheritance/)
  for single inheritance by allowing struct inheritance and
  traits to extend structs). This RFC can be considered to take the object
  representation scheme from RFC 9 with a different mechanism for inheritance.

* [Extending enums (11)](https://github.com/rust-lang/rfcs/pull/11) Proposes
  combining enums and structs in a similar, but not identical way to this RFC.
  Introduces `impl ... as match` and `impl ... use ...` to handle method
  dispatch.

* [Unify and nest enums and structs (24)](https://github.com/rust-lang/rfcs/pull/24)
  A variation of RFC 11, superseeded by this RFC.

* [Trait based inheritance (223)](https://github.com/rust-lang/rfcs/pull/223)
  "This is largely based upon #9 and #91, but fleshed out to make an actual
  inheritance proposal.". I'm afraid I haven't spent enough time on this to give
  an accurate summary.

* Kimundi and eddyb have promised an RFC for a possible solution using trait
  fields.


## Variation - `data` for `struct` and `enum`

An alternative to using `enum` and `struct` is to use a single keyword for both
constructs. `data` is my personal favourite and matches Haskell (I'm not aware
of other uses, Scala?). We would then need some way to indicate the sized-ness
of the datatype. The obvious way is to use another keyword. For DST we use
`Sized?` but this is not a keyword, it indicates the possible absence of the
default trait bound `Sized`, so that is probably not suitable. Furthermore, I'm
not sure whether the sized or unsized version should be the default.

We could then either forbid the use of `enum` and `struct` or we could allow
them as syntactic sugar for `sized data` and `unsized virtual data`,
respectively.

# Unresolved questions

## Initialisation

To initialise a struct you must give values for all its fields. There is a
technical and an ergonomic problem here: if the base struct is in a different
module, then its private fields cannot be named in the constructor for the
derived struct; and if the base struct has a lot of fields, it is painful and
error-prone to write out their values in multiple places.

We can address the first problem by adjusting the privacy rules to always allow
the naming of private fields in constructors if the most derived struct's fields
are visible.

To address the second problem, we start by adjusting the rules for struct
initialisers. An initialiser currently has the form
`Foo { f_0: value_0, ..., f_n: value_n, .. e }` where `e` is an expression with
type `Foo` and which supplies any fields not in the field list. We can make this
more general by accepting an expression with type `Foo` or any of its base
structs, where the programmer must explicitly give at least any missing fields.
This addresses both the first and second problems described above. However, it
has a problem of its own if the base struct is virtual - then we cannot
create an instance with the required type, so the derived struct is impossible
to instantiate.

I don't see any good way to solve this problem. Here are some ideas (I think the
second or third are my favourites):

* Where a struct is virtual, allow the struct to be instantiated, but do not
  allow any method calls on such objects, nor taking its address. The only
  operations allowed on objects with such type are field access and use in
  initialiser expressions. This is yet more added complexity.

* Some attribute similar to `deriving` for a virtual base struct which
  automatically generates a non-virtual derived struct with no extra fields and
  with an implementation which `fail!`s for every method call. A constructor for
  the base struct can create one of these objects and return a `Box<Foo>` (if
  `Foo` is the base struct). We would adjust struct initialisers to allow
  `Box<T>` as well as `T` where `T` is the struct being initialised or any of
  its base structs. This can be thought of as a dynamic version of the above
  (static) proposal. It has the advantage of much less compiler complexity and
  no new language rules (ish), however, this comes at the expense of risk of
  runtime failure.

* Allow fields to have default values, e.g., `struct Foo { x: int, y:int = 42 }`,
  here `y` has a default. When instantiating `Foo`, `x` must be provided and `y`
  may be. If `y` is provided then it overrides the default value. If a struct is
  to have derived structs in different modules, then all private fields must
  have defaults (this does not need to be enforced by the compiler - it is a
  natural consequence). This has the disadvantage that fields can only ever have
  a single default (as opposed to using multiple constructor functions) and
  there can be no input to field defaults. It is also more language complexity,
  but I believe this is useful in regular structs too.


## Multiple inheritance

Do we need multiple inheritance? We _could_ add it, but there are lots of design
and implementation issues. An example use case for multiple inheritance (from bz) is
that some DOM nodes require mixin-style use of classes which currently use
multiple inheritance, e.g., nsIConstraintValidation.

Example with traits:

```
impl Element {
    virtual fn bar() -> uint;
}

trait NSICompositor {
    fn x() -> uint;
    fn y() -> uint;
    fn bar() -> uint { self.x() + self.y() }
}

impl NSICompositor for Element1 {
    fn x() -> uint { self.x }
    fn y() -> uint { self.y }
}

impl Element1 {
    override fn bar() -> uint { NSICompositor::bar(self) }
}

impl NSICompositor for Element2 {
    fn x() -> uint { self.x }
    fn y() -> uint { self.y }
}

impl Element2 {
    override fn bar() -> uint { NSICompositor::bar(self) }
}
```

I believe that all such uses can be implemented using the traits mechanisms in
Rust and that these will interact cleanly with the rest of this RFC. Therefore,
we should not add any additional mechanism for multiple inheritance.


# Staging

There is a lot to implement here. Here is an approximate implementation plan:

* Add enum variants to the type name space (this is backwards incompatible so
must happen before 1.0);
* Ensure the syntax for struct inheritance matches the syntax in this RFC for
non-lexical nesting;
* Allow private fields to be initialised in the initialisers of sub-structs
(possibly remove this later);
* Implement the rules around instantiation of virtual structs;
* Add vtables for virtual structs and ensure virtual structs behave as DST
unsized types;
* Add coercions for virtual structs;
* Implement trait matching rules;
* Implement rules for Drop and destructors;
* Implement virtual impls;
* Implement pattern matching downcasts for virtual structs (at this stage we
have everything Servo needs for the DOM);
* Implement initialisation mechanism;
* Allow lexically nested structs and deeply nested enums. Refactor the compiler
middle-end to have a single representation;
* Allow use of enum variants as types.
