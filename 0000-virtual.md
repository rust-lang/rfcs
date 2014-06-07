- Start Date:
- RFC PR #: 
- Rust Issue #: 

# Summary

Efficient single inheritance via virtual structs and unification and nesting of
structs and enums.

Allow inheritance between structs and virtual methods on struct objects. This
would support data structures such as the DOM which need to be efficient both in
speed (particularly non-virtual field access) and space (thin pointers).

This approach unifies many of our data types so although we add features
(virtual methods, nested enums), we reduce complexity of the language and
compiler in other directions.

# Motivation

Supporting efficient, heterogeneous data structures such as the DOM. Precisely
we need a form of code sharing which satisfies the following constraints:

* Cheap field access from internal methods;
* Cheap dynamic dispatch of methods;
* Cheap downcasting;
* Thin pointers;
* Sharing of fields and methods between definitions;
* Safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

Example (in pseudo-code):

```
class Element {
    Element parent, left-sibling, right-sibling;
    Element[] children;

    foo();

    template() {
        x = foo();
        ...
    }
}

class Element1 : Element {
    Data some-data;

    template() {
        return some-data;
    }
}

final class Element2 : Element {
    ...
}
```

# Detailed design

Syntactically, we unify structs and enums (but not the keywords) and allow
nesting. That means enums may have fields and structs may have variants. The
keyword (`struct` or `enum`) is only required at the top level. Unnamed fields
(tuple variants/tuple structs) are only allowed in leaf data. All existing uses
are preserved. Some examples:

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

Open question: should we use `()` or `{}` when instantiating items with a mix of
named and unnamed fields? Or allow either? Or forbid items having both kinds of
fields.

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

All leaf variants may be instantiated. No non-leaf enums may be instantiated
(e.g., you can't create an `E3` or `VariantNest` object). By default, all
structs can be instantiated. Structs, including nested structs, but not leaf
structs, may be marked `virtual`, which means they cannot be instantiated. Put
another way, enums are virtual by default. E.g., `virtual struct S1 { ... S2 {
... } }` means `S1` cannot be instantiated, but `S2` can. `virtual struct S1 {
... virtual S2 { ... } }` would mean `S2` could not be instantiated, but would
be illegal because it is a leaf item. The `virtual` keyword can only be used at
the top level or inside another `virtual` struct.

Open question: is the above use of the `virtual` keyword a good idea? We could
use `abstract` instead (some people do not like `virtual` in general, and this
use is different from the use described below for methods). Alternatively, we
could allow instantiation of all structs (unless they have pure virtual methods,
see below) or only allow instantiation of leaf structs.

We allow logical nesting without lexical nesting by using `:`. In this case a
keyword (`struct` or `enum`) is required and must match the outer item. For
example, `struct S3 : S1 { ... }` adds another case to the `S1` defintion above
and objects of `S3` would inherit the fields `f1` and `f2`. Likewise, one could
write `enum Variant3 : E3;` to add a case to the defintion of `E3`. Such items
are only allowed in the same module, or a sub-module of, the outer item. Why?

    1. Prevents people from abusing virtual structs to create an open-ended
    abstraction: traits are more suitable in such cases.
    2. Downcasting is more optimizable, becomes O(1) instead of O(n). This is a
    common complaint against C++ RTTI (as pointed out on the mailing list).
    3. Addresses the private fields initialization gotcha. (Without this
    restriction, it is not clear how to initialise a struct with private fields 
    in a different module).

When matching data types, you can use any names from any level of nesting to
cover all inner levels. E.g.,

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
or reference to an enum object is just a regular pointer to a regular enum
object. Nested variants should use a single tag and the largest variant must
take into account nesting. Event if we know the static type restricts us to a
small object, we must assume it could be a larger variant. That allows for
trivial coercions from nested variants to outer variants. We could optimise this
later, perhaps.

Non-leaf struct values are unsized, that is they follow the rules for DSTs. You
cannot use non-leaf structs as value types, only pointers to such types. E.g.,
(given the definition of `S1` above) one can write `x: S2` (since it is a leaf
struct), `x: &S2`, and `x: &S1`, but not `x: S1`. Struct values have their
minimal size (i.e., their size does not take into account other variants). This
is also current behaviour. Pointers to structs are DST pointers, but are not
fat. They point to a pointer to a vtable, followed by the data in the struct.
The vtable pointer allows identification of the struct variant.

To summarise the important differences between enums and structs: enum objects
may be passed by value where an outer enum type is expected. Struct objects may
only be passed by reference (borrowed reference, or any kind of smart or built-
in pointer). enum values have the size of the largest variant plus a
discriminator (modulo optimisation). Struct values have their minimal size. For
example,

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
data is secondary and discrimination is primary, for example the current
`Result` type. Structs would be used for large or diversely sized objects where
discrimination is secondary, that is they are often used in a polymorphic
setting, a good candidate would be the AST enum in libsyntax (or of course, the
DOM in Servo).

Matching struct objects (that is pointer-to-structs) takes into account the
dynamic type given by the vtable pointer and thus allows for safe and efficient
downcasting.

Methods may be marked as `virtual` which allows them to be overridden in the
sub- struct's impl. Overriding methods must be marked `override`. It is an error
for a method to override without the annotation, or for an annotated method not
to override a super-struct method. (Methods marked `override` but not `virtual`
may not be overriden). Virtual methods may be given without a body, these are
pure virtual in the C++ terminology. This is only allowed if the struct is also
marked `virtual`. Non virtual methods will be statically dispatched as they are
currently. Virtual methods are dispatched dynamically using an object's vtable.
Methods may be marked as both `override` and `virtual` to indicate that override
and may in turn be overridden. A method without the `virtual` annotation is
final (in the Java sense) and may not be overridden.

Open question: alternative to `virtual` keyword - `dynamic`.

## Subtyping and coercion

Nothing in this RFC introduces subtyping.

Inner enum values can implicitly coerce to outer enum values.

Inner struct pointer values can impliciitly coerce to outer struct pointer
values. Note that there is no coercion between struct values. Since all but leaf
structs are unsized, they may not dereferenced. Thus we are immune to the object
slicing problem from C++.

Via the DST rules, it should fall out that these coercions work for smart
pointers as well as `&` and `Box` pointers.

Note that this means if `R` is an inner struct of `S` and `S` implements a trait
`T`, but `R` does not, then given a pointer to an `R` object, it may be coerced
to an `S` in order to call methods defined in `T`, if the self type of those
methods is a pointer to self (e.g., `&self`).

## Generics

(I feel the syntax could be nicer here, any ideas?)

Nested items must specify formal and actual type parameters. The outer items
type parameters must be given in `<>` after a `:` (similar to the inheritance
notation, but no need to name the outer item). E.g.,

```
struct Sg<X, Y> {
    Sgn<X, Y, Z> : <X, Y> {
        field: Foo<X, Z>
    }
}

let x = Sgn<int, int, int> { field: ... };
```

In the nested notation only, if an item has exactly the same type parameters as
its parent, they may be ommitted. That is for

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
actual type parameters for the parent. E.g.,

```
struct Sg<X, Y> {}

struct Sgn<X, Y, Z> : Sg<X, Y> {
    field: Foo<X, Z>
}

let x = Sgn<int, int, int> { field: ... };
```

## Privacy

The privacy rules for fields remain unchanged. Nested items inherit their
privacy from their parent, so module private by default unless the parent is
marked `pub`.

Open question: is there a use case for allowing nested items to be marked `pub`?
That is having a private parent but public child. What about the opposite?

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
across in a tutorial. On the other hand they are satisfying different use cases
with different priorities. I believe the extra complexity does not need to be
paid for by every user in the sense that, unless you specifically want to use
these features, you don't need to know about them.

# Alternatives

There have been many proposals for alternative designs and variations on this
design. One minor variation would be to use anonymous fields rather than `:`
extension for struct inheritance. An alternative proposal is to allow traits to
extend a single struct and add subtyping appropriately. We would then need to
add support for some kind of RTTI (possibly using a trait and macros) to allow
safe and efficient downcasting.

## Some previous RFCs

* [Virtual Structs (5)](https://github.com/rust-lang/rfcs/pull/5) Stays as
  closely as possible to inheritance schemes in Java or C++. Touches only
  structs so does not unify structs and enums. That means we end up with two
  design choices, where there probably shouldn't be. The scheme for defining
  virtual methods is used in this RFC>

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
  combining enums and structs in a similar, but not identical to this RFC.
  Introduces `impl ... as match` and `impl ... use ...` to handle method
  dispatch.

* [Unify and nest enums and structs (24)](https://github.com/rust-lang/rfcs/pull/24)
  A variation of RFC 11, superseeded by this RFC.


# Unresolved questions

## Multiple inheritance

Do we need multiple inheritance? We _could_ add it, but there are lots of design
and implementation issues. The use case for multiple inheritance (from bz) is
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

## Drop

What to do about (virtual) destructors? I feel the C++ approach is too much of a
foot gun. By limiting struct inheritance to a module, we should always be able to
infer whether or not a destructor is virtual. Need to work out how exactly
implementing the drop trait interacts with inheritance.

We need to cope with the situation where a struct object with static type T1 and
dynamic type T2 goes out of scope and T2 implements `Drop` and T1 doesn't - we
still need to call T2::drop. One solution could be that if an inner struct
implements `Drop` then so must the outer struct. Calling `drop` is then just a
regular virtual call and is only necessary if the static type implements `Drop`.

A generalisation of this is should we have a mechanism for requiring inner items
to implement a trait (with or without implementing it in the outer item, the
former case is like saying "must override")? This is kind of dual to the idea
above that if an outer item implements a trait, then the inner trait appears to
implement it too, via coercion. (ht Niko).

Should we automatically call drop on super-structs? Or rely on the programmer to
do that manually?

### Straw man proposal

Allow `virtual impl Tr for T;` syntax where `T` must be a struct or enum and
which has the semantics that any inner item of `T` must provide an implmentation
of `Tr`. Similarly to pure virtual methods, this implies that `T` cannot be
instantiated.

Traits may be marked `inherit` (this is a terrible keyword, anyone got any
better ideas? I guess we could use `virtual` here too): `inherit Trait Tr
{...}`. This implies that for an item `T` to implement `Tr` any outer item of
`T` must also implement `Tr` (possibly providing a pure virtual impl). This is
checked where the impl is declared, so it is possible that an impl could be
declared for an outer item in a different module but due to the visibility
rules, it is invisible, this would be a compile error. Since `impl`s are not
imported, only traits, I believe this means that if a trait is inherit, then
anywhere an implementation for an inner item is visible, then an implementation
for the outer item is also visible.

Drop is marked `inherit`.

It is the programmer's responsibility to call `drop()` for outer-items from the
impl for the inner item, if necessary.

I believe that this gives the desired behaviour and is backwards compatible,
other than the addition of the `virtual` and `inherit` keywords.

## Calling overridden methods

If a method is overridden, we should still be able to call it. C++ uses `::`
syntax to allow this, UFCS should let us do this. Since all such uses would use
static dispatch, we would use self-as-arg syntax, e.g.,
`BaseType::method(self, ...)`.
