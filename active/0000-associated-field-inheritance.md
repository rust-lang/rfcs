- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Solving  the servo DOM design requirements through a combination of a few  orthogonal features and extensions that are useful on their own:

- Allowing upcasting of a trait object
- Extend trait objects to include associated fields that map to fields of the implementing type
- Allow opt-in use of an internal vtable for an trait object hierarchy.
- Provide composition sugar for struct fields, and make it usable for associated fields too.
- Ability to call the default implementation of a trait method, even if a implementer overrides it.
- Ability to override associated items of a trait with default impls in a sub trait.

# Motivation

The  features mentioned above make it possible to implement the Node  hierarchy in the DOM  with trait inheritance and trait objects,  fulfilling the given design constraints:

* cheap field access from internal methods;
  - Given due to monomorphization.
* cheap dynamic dispatch of methods;
  - Initial call through the vtable, starting from there statically dispatched due to
    monomorphization.
* cheap downcasting;
   - Downcasting can be achieved by providing individual methods on super  traits that return `Option<&SubTrait>` or similar, optionally  inheriting from `Any` to allow downcasting to the concrete impl type  without adding a method for each to the root Trait. The cost of a  downcast would be a single virtual call to a trivially implemented  function that either returns `None` or `Some(self)`.
* thin pointers;
  - Provided by marking a given trait object as using an internal vtable
* sharing of fields and methods between definitions;
  - Methods are shared through default methods and trait inheritance
  - Fields are shared by using the field composition sugar to copy their definition into a given
    struct/trait without needing to enumerate them more than once.
* safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.
  - At most a possible library implementation of the downcasting feature needs a bit of unsafe
    code internally. Apart from that, the feature mostly depend on compiler checked and
    generated code, like the trait object upcasting and the internal vtable.
* Overridable methods for each step in the inheritance hierarchy.
   - Overridable/refine-able default methods in sub traits in combination  with the ability to call super trait default method impls allows the  classic OOP pattern of overriding a method and implementing it in terms  of the parent classes method.

There  are already a number of alternative proposal for how to solve these  issues, attacking the problem from a number of different angles:
- Adding inheritance to regular structs, introducing virtual structs and virtual methods in the process.
- Making types with the same memory layout coercible to each other.
- Solving it with trait objects, but only enabling single inheritance

The core motivation for this particular variant is the opinion that many of these options have
one of these problems:

-  Adding a single inheritance-analog to types enables a second way to  provide dynamically extensible interfaces apart from traits. This has  the following issues:
  - People coming from other mainstream languages are usually familiar with type inheritance,
    which will make them overuse this feature in places where a trait-based design would be
    the better choice.
  - As the language grows, this can lead to a split in the library ecosystem between APIs
    using single inheritance-based interfaces, and those using trait-based interfaces.
  - These types still need to support trait implementations, which in the presence of trait
    inheritance, trait objects, DST and the existing coercion rules makes combining them
    potentially complex.
- Only focusing on single inheritance
  - The DOM requirements only need single data and interface inheritance, and this RFC
    only aims to provide that, seeing how single inheritance has known, good performance
    characteristics in the dynamic case.
    However, multiple inheritance is useful for certain tasks, as its use for traits shows, and in
    the case of static dispatch most of the issues of multiple inheritance for data don't exist
    either, so forcing a core distinction in the language rather than just forbidding problematic
    patterns seems like a too conservative step to take.

# Detailed design

Because each feature necessary for the DOM implementation is basically a separate RFC that is useful on its own, they are listed as such here, with a concrete example implementation of the DOM afterwards.

## Upcasting

### Upcasting Summary

Change  the vtable layout of trait objects to something embedding all super trait  vtables in prefix or prefix+constant offset position. Implement  upcasting of a trait object in the language by a constant pointer offset of the vtable.

### Upcasting Motivation

Right  now, for each trait object for a type, rustc generates a vtable  containing the drop glue for the type, and all methods of the trait and  its super traits. This makes it possible to call all methods of super  traits on the trait object, and is space optimal as every method is  contained in the vtable only once, but it offers no cheap way to compute  the address of the vtable of a parent trait from an individual  instance.

This means you can not easily treat different trait objects with common ancestor traits the same way, or reuse a implementation for a parent trait object in a child trait object

### Upcasting Detailed Design

Change the current vtable generation/layout algorithm to this one:


```
// In psydo code:
vtable_layout(t: Trait) -> [VtableElement] {
    let vtable = []
    if t.supertraits.is_empty {
        vtable = [drop_glue] // plus maybe alignment and size
    } else {
        for all st in t.supertraits {
            vtable ++= vtable_layout(st)
        }
    }
    for all associated_item in t.associated_items {
        vtable ++= [associated_item]
        // associated_item is a method,
        // a associated constant,
        // or a field offset (see other sub RFC)
    }
    return vtable
}
```

This  has the effect that every supertrait is embedded in the vtable of a  given trait, at the cost of slight bloat and duplication for every case  of multiple inheritance in a trait hierarchy. For single  inheritance-only hierarchies, this algorithm will generate the same  vtable elements as today, just reordered in prefix order, and thus will  have zero additional bloat.

Once  this is done, the compiler needs to allow casts or coercions like  `&Trait as &SuperTrait` and implement it by adding a constant  pointer offset to the vtable pointer.

Again,  for trait hierarchies that only employ single inheritance, any upcast  will always add a constant zero offset, and thus be a no-op.

As  an optional last step, implement an llvm optimization pass that  de-duplicates all vtables in a compilation unit by making standalone  vtables of supertraits be pointers into the vtables of subtraits  instead. This would have the potential of turning the additional code  bloat into a size win instead, as most vtables in a trait hierarchy  would then share the same memory location instead of being individual  constants.

### Upcasting Drawbacks

Due  to the embedding of super trait vtables, certain features in a trait  hierarchy will cause the same elements to be contained multiple times in  the table:

-  For each case of multiple inheritance, the drop_glue function pointer  will duplicate proportional to the number of parent traits.
-  For each case of diamond inheritance, all elements of the diamonds root  vtables will duplicate proportional to the number of inheritance paths  leading to the diamond root.

However, this causes no restrictions apart from non-optimal space utilization

### Upcasting Alternatives

- Opt-in to upcasting by having a unsized lang item wrapper: `t as &Upcastable<Trait>` instead of `t as &Trait`. It would work the same way as described above, with regular trait remaining the way they are now.
- Implementing upcasting manually on a case by case basis with something like this:
```
trait SubTrait: SuperTrait {
    fn as_super_trait(&self) -> &SuperTrait { self }
}
```
But this has the disadvantage of being hard-coded to a specific pointer type and needing a virtual call to cast.

## Associated fields

### Associated fields Summary

Add the ability to define associated fields in traits, which are named fields that need to be mapped to a field (or rather, a lvalue) derived from the implementing type:

```
trait Foo {
    a: uint
}

struct MyStruct {
    x: uint
}
impl Foo for MyStruct {
    a: uint => self.x
}

struct MyTuple(uint);
impl Foo for MyStruct {
    a: uint => self.0
}

impl Foo for uint {
    a: uint => self
}

// For example, use with a trait object like this:
fn main() {
    let foo: &Foo = &MyTuple(100);
    assert_eq!(foo.a, 100);
}
```

### Associated fields Motivation

Today, if a type implements a trait, that expresses that the type belongs to a group of types that share certain common aspects defined by the trait:

- Trait implementations in the form of trait inheritance
- Types in the form of associated types
- Behavior and computed Data in the form of associated functions/methods
- Static Data in the form of associated statics

However, there is a case that is not well covered with these options: Abstracting over pieces of dynamic data embedded in the implementing type directly. You can emulate it somewhat with getters and setters and rely on inlining to remove the function call costs, but that is more inconvenient and fragile than just being provided with a lvalue directly, and in the case of trait objects there will always be the cost of dynamic dispatch. Associated fields would solve this by enabling to be generic over a type that provides certain types as fields.

So in short:
- Ability to abstract over data instead of behavior.
- Cheap access to data with trait objects

### Associated fields Detailed Design

The grammar for trait body definitions gets extended with the production rules of struct fields.

The grammar for impls gets extended with the production rules of fields, followed by `=> CONSTEXPR` where the constexpr can use the `self` keyword to refer to a value of the Self type. The constexpr needs to refer to a lvalue, usually a field of `self`, or a field of a field of `self`, or even `self` itself.

In the case of a trait impl, the set of fields has to match the trait definition as usual, and in the case of an inherent impls you are free to define arbitrary mappings.

Associated fields for traits follow the same scoping rules as trait methods as far as the `.foo` syntax is concerned: If the trait is in scope and the type is known to have a bound on it, or is a trait object, they can be referred to directly just like a regular field.

To make their use memory safe, the compiler has to ensure that you can not get aliasing paths to the same physical field through different associated fields or the real field. The exact mechanism for this still needs to be thought through, but a possible rule could be to enforce that each field association in a impl needs to be non-overlapping with any other, and forbid simultaneous use of an associated field and the real field it points to in statically dispatched code (which is the only situation where both are visible at the same time).

In the general dynamic case, the fields will be stored as offsets in the vtable, but there is a way to optionally elide those:

A `#[repr(fixed)]` attribute will be introduced that can be applied to a _trait_ definition:

```
#[repr(fixed)]
trait Foo {
    a: uint,
    b: u8
}

#[repr(fixed)]
trait Bar: Foo {
    c: String
}
```

It forces all implementors of the trait to order to lay out their fields in a fixed prefix order according to the trait hierarchy and the field mappings in the impl:

```
// Given these impls:
impl Foo for X { a: uint => self.a, b: u8 => self.b }
impl Bar for X { c: String => self.c }

// OK:
#[repr(fixed)]
struct X {
     a: uint,
     b: u8,
     c: String
}

// ERROR, undefined field order:
struct X {
     a: uint,
     b: u8,
     c: String
}

// ERROR, wrong field order:
#[repr(fixed)]
struct X {
     b: u8,
     a: uint,
     c: String
}
```

The annotation is per-trait to  allow opting-out of it again at the leaf regions of a trait hierarchy,  but once you start leaving it off you can not enable it again for traits further down in the inheritance hierarchy.

Additionally, in the case of multiple inheritance, only one parent trait may use this optimization - basically only one single inheritance path is allowed to make use of it in the trait hierarchy.

### Associated fields Drawbacks

- This adds another complicated aspect to the trait system
- Mapping an associated field to a real field is a novel concept and hence not used nowhere else in Rust

### Associated fields Alternatives

- Deal with abstraction over data as today: Getters, Setter, and the need of virtual calls for trait objects.
- Make mapping not freely choosable, but rather require appropriately named fields to be present in the struct (possibly in the right order as well)

## Overridable default items

### Overridable default items Summary

Add a `override` keyword and allow overriding associated items of parent traits with a default implementation in a sub trait.

### Overridable default items Motivation

Today, a trait defines associated items that can either be implemented per default in the trait definition in terms of its other associated items and parent traits, or by explicitly implementing them for a  concrete type in terms of all its implementations.

However, there are sometimes situations where a child trait has enough information about the Self type that it could provide a generic default implementation for items defined in parent traits, without requiring an concrete impl.

One example from the std library are the Ord and Eq traits: If a type implements PartialOrd, it has enough information to provide a impl for PartialEq, but the current trait system forces you to write an explicit impl:

```
trait PartialEq {
    fn eq(&self, other: &Self) -> bool;
}

trait PartialOrd: PartialEq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>;
}

struct Foo(uint);
impl PartialOrd for Foo {
    fn partial_cmp(&self, other: &Foo) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl PartialEq for Foo {
     fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Equal)
    }
}
```

With the ability to override parent trait items with a default implementation, these traits could instead be expressed like this:

```
trait PartialEq {
    fn eq(&self, other: &Self) -> bool;
}

trait PartialOrd: PartialEq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>;

    override fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Equal)
    }
}

struct Foo(uint);
impl PartialOrd for Foo {
    fn partial_cmp(&self, other: &Foo) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl PartialEq for Foo {}
```

Note the empty `PartialEq` impl: The `eq` method is now provided in terms of a default method located in a child trait definition.

In combination with explicitly callable default methods (See other sub RFC), the same mechanism also allows to "specialize" existing default methods by overriding them with a method that calls out to the default method of the parent trait. This, in combination with trait objects, enables a OOP-like overriding of methods on each step of the inheritance hierarchy:

```
trait Base {
    fn add(&mut self, uint);
    fn mul(&mut self, uint);
    fn calc(&mut self) {}
}

trait Add2Decorator: Base {
    override fn calc(&mut self) {
        Base::default::calc(self);
        self.add(2);
    }
}

trait Mul5Decorator: Add2Decorator {
    override fn calc(&mut self) {
        Add2Decorator::default::calc(self);
        self.mul(5);
    }
}
```

### Overridable default items Detailed Design

- Introduce the `override` keyword, and allow it on associated items that provide default implementations.
- If a type implements a trait with overridden default implementations, it is allowed to leave them of in the impl of the parent trait, in which case the items implementation is translated from the child traits item.
- A type is not allowed to implement two traits that each override the same item and are not in a inherits-from relationship (diamond problem)
- Overridden default implementations do not affect explicitly callable default implementations, those still refer to the original one.

### Overridable default items Drawbacks

This makes it somewhat harder to find out where a given associated items impl comes from

### Overridable default items Alternatives

- Instead of a `override` keyword, this could also be a `#[override]` attribute. However, because overriding has a distinct different semantic compared to defining a new associated item, this seems like a misleadingly weak way to differentiate the two.

- The choice of `override` as a keyword could be changed to something different, like the existing `super`.

## Internal vtable

### Internal vtable Summary

(Requires associated fields and overridable default items)

(See alternative section of this sub RFC for drastically reducing the user facing machinery of this design)

Add these lang items and attributes:

- `struct Vtable<T, Sized? Tr>` a lang item type that represents the vtable of a given type-trait combination in a type safe way.
- `intrinsic fn get_vtable<T, Sized? Tr>() -> Vtable<T, Tr>` a intrinsic for constructing such a Vtable. Only type checks if `T` actually implements `Tr`.
- `trait IsVtable<T, Sized? Tr2>`: lang item that is automatically implemented for all `Vtable<T, Tr>` where `Tr2`s vtable is a prefix of `Tr`s. This is basically a small-scale variations of the coercible RFC and used to model a sub-typing relationship between upcastable vtables.
- `#[repr(internal_vtable)]` attribute to allow defining a trait as having an internal vtable as its first field. Any trait object marked as such will not be a fat pointer but a thin pointer, as method dispatch refers to the vtable stored inline in the object.

### Internal vtable Motivation

With associated fields and upcastable trait objects, all the the pieces are there to model  OOP-like multiple inheritance classes with trait objects.

However, using them that way still incurs the size costs of fat pointers, which makes them unsuitable for situations with special size and performance constraints like the servo DOM.

Internal vtables are a way to explicitly opt-in trait hierarchies into storing the vtable of trait objects inline in the object, partially restricting them to single-inheritance in the process but allowing them to be thin pointers with vtable look-ups redirected into the first field of the data pointer.

Additionally, the resulting layout is close enough to how C++ compilers usually represent non-virtual-inheriting classes that it opens the door to partial C++ FFI.

### Internal vtable Detailed Design

- Add a `Vtable<T, Sized? Trait>` lang item type that acts as a type-safe wrapper around the static pointer pointing at that vtable. (Optionally, make each entry of the vtable reachable as a unsafe field of a function pointer to allow very low-level hacks. This is not required for this proposal, however)
- Add a `IsVtable<T, Sized? Trait2>` lang item marker trait that any `Vtable<T, Sized? Trait>` implements if `Trait2`s vtable is a prefix of `Traits`. This is known to be the case in two situations:
  - Only single inheritance is used between those two traits
  - The traits have `#[repr(inline_vtable)]` annotations, which forces them to be prefix in the vtable
- Add a `get_vtable<T, Sized? Trait>() -> Vtable<T, Trait>` intrinsic that can only be called for type-trait combinations with a "implements" relation, and that returns the appropriate Vtable.
- Add a `#[repr(inline_vtable)]` attribute that applies to a trait definition and places the following restrictions on the trait `Tr` and its implementing type:
  - `Tr` is required to contain an leading associated field with a type that has the `IsVtable<Self, Tr>` bound.
  - The implementing type is required to map the vtable field to its first field, which is a sub case of what `#[repr(fixed)]` enforces. (And hence both can be combined)
  - The implementing type is not allowed to implement traits from more than one inline-vtable trait hierarchy.

  Additionally, any descendant `SubTr` of `Tr` that wishes to opt-in to the internal vtable with `#[repr(inline_vtable)]` as well has the following restrictions:

  - `SubTr` is required to have a `IsVtable<Self, SubTr>` bound on the type of the vtable field
  - `SubTr` is required to inherit from only one trait with `#[repr(fixed, inline_vtable)]` annotation

These rules result in any type that implements parts of a trait hierarchy marked with `#[repr(inline_vtable)]` being required to contain a leading `Vtable` field corresponding to the most specific trait of the hierarchy it implements, which will contain the vtables of all of its ancestors as a prefix.

#### Casting

To  make casting and upcasting work, vtables in general need to be defined as  containing the sub vtables of inline-vtable parents in  prefix position.  Because any trait  can only have at most one  inline-vtable parent, this  is always possible.

- Casting the concrete pointer-to-struct to an inline vtable trait object can be implemented as a no-op transmute to the trait object, as the struct is already guaranteed to start with an vtable that has the required vtable as a prefix.
- Upcasting a inline-vtable trait object to a parent that is also inline-vtable is a no-op for the same reasons.
- Upcasting a inline-vtable trait object to a parent that is not inline-vtable involves loading the vtable from inside the data pointer to create a fat pointer, then the same constant-pointer-offset transformation from the upcasting RFC applies.
- Upcasting a regular fat trait object to a inline-vtable parent just throws away the outer vtable, as the struct is guaranteed to contain the most specific vtable that is inline-vtable.

### Internal vtable Drawbacks

The mechanism and enforced rules are fairly complex. However, seeing how this a rarely-needed optimization that seems tolerable.

### Internal vtable Alternatives

The user facing complexity could possibly be hidden away by integrating the concept of a internal vtable more into the language:

- Only a single `#[repr(internal_vtable)]` attribute is added that can be applied to both traits and structs
- If applied to traits, it causes the same requirements on implementing structs to contain a leading vtable field for the most specific trait in the implemented hierarchy
- If applied to structs, it causes the generation of a hidden, leading field containing the Vtable of the most-specific implemented inline-vtable trait.
- If such a struct is constructed, its hidden leading vtable field is automatically initialized with the correct vtable pointer.
- There is still the restriction that you only can have at most one inline-vtable parent trait per trait in the hierarchy
- But apart from that the only user facing restriction would be to be required to put a `#[repr(inline_vtable)]` attribute on any struct that implements a trait that has the same attribute.

## Field composition sugar

### Field composition sugar Summary

Add syntax for reusing the fields of an struct in another structs definition, without any additional semantic meaning beyond that:

```
struct A {
    a: uint,
    b: uint,
}
struct C {
   x: uint,
   ..A,
   c: uint,
}
```

would be equivalent to

```
struct A {
    a: uint,
    b: uint,
}
struct C {
    x: uint,
    a: uint,
    b: uint,
    c: uint,
}
```

### Field composition sugar Motivation

Today, if you want different types to share a common field sub structure, there are two options:
- Copying the fields of the common struct into your struct, violating the DRY principle.
- Embedding a struct with common fields as a field in your struct, allowing you to only need to mention the type name in your struct definition. But this requires you to prefix all access to them with that field name, and gives you more than asked for by allowing you to access the common structs impls.

Field composition sugar fills the use case between those two options by allowing to share structure directly, without embedding of another type as a named field.

### Field composition sugar Detailed Design

The grammar for struct fields is extended with the production rule for `.. PATH`

During compilation, every use of that syntax basically desugars to a flat list of fields defined by the referenced struct.

If the referenced struct is generic, it needs to have all type arguments applied. It then desugars to fields with the original type arguments substituted with the applied ones:

```
struct X<T, U> {
    x: T
    y: U
}

struct Z<T> {
    ..X<T, uint>
}

/* desugars to: */

struct Z<T> {
    x: T,
    y: uint
}
```

### Field composition sugar Drawbacks

More syntax to keep track of

### Field composition sugar Alternatives

Rely on the two existing options

## Explicitly callable default method impls

### Explicitly callable default method impls Summary

Add  a way to explicitly invoke the default implementation of a trait method  for a implementing type, even if that type has overridden the default  method with its own behavior. Provide special syntax to differentiate  the two.

### Explicitly callable default method impls Motivation

Today,  you can define default associated functions in a trait, and override  them for each specific implementation. Eg, a simplified example would be  the way the Visitor traits in the compiler work right now:

```
trait Visitor {
    fn visit_foo(&mut self, foo: &mut Foo) {
        foo.visit_bar(&foo.bar);
    }
    ...
}

impl Visitor for X {
    fn visit_foo(&mut self, foo: &mut Foo) {
        foo.bar.change_something();
        foo.visit_bar(&foo.bar);
    }
}
```

However,  once overridden you have no way to refer to the content of the default  implementation of the method, which is unfortunate because in this case  it hinders code reuse of the default implemented behavior:

```
impl Visitor for X {
    fn visit_foo(&mut self, foo: &mut Foo) {
        foo.bar.change_something();
        /* invoke the default behavior */
    }
}
```

Today  you can work around this on a case-by-case basis by making the default  impl call a public standalone function and referring to it in the impls,  but this has disadvantages, see Alternatives section.

As  a general solution, there needs to be a way to refer to the default  implementation directly, eg with a special keyword modifier in UFC  syntax: `<Type as Trait>::default::method()`

### Explicitly callable default method impls Detailed Design

-  Allow syntax to construct a path to default-implemented items of a  trait, eg by making `default` a keyword and making `path::default::item`  parse.
-  For every reference to such an default item for a given type, translate  the code the same way as it would if the impl for that trait would not  override the default.

This should work for any associated item that can have a default, like functions/methods, types, statics, etc.

### Explicitly callable default method impls Drawbacks

It  complicates the part grammar somewhat, and is potentially confusing in  regard to trait inheritance, as depending on syntax chosen "selecting  the default item" could be confused with "selecting a parent trait  item".

### Explicitly callable default method impls Alternatives

The syntax is totally open for bike shedding. Possible candidates are

```
<Type as Trait>::super::method();
<Type as Trait>::self::method();
<Type as Trait>::default::method();
<Type as Trait>::trait::method();

<Type as Trait>::method::super();
...
```

You  can work around the lack of directly callable default implementations  today by providing them as  standalone  functions, which is a pattern  that is used at a few places in  the  compiler:

```
fn default_visit_foo<V: Visitor>(&mut T, foo: &mut Foo) {
    foo.visit_bar(&foo.bar);
}

trait Visitor {
    fn visit_foo(&mut self, foo: &mut Foo) {
        default_visit_foo(self, foo)
    }
    ...
}

impl Visitor for X {
    fn visit_foo(&mut self, foo: &mut Foo) {
        foo.bar.change_something();
        default_visit_foo(self, foo)
    }
}
```

However,   this is verbose and can lead to bugs (eg, the compiler used to have a   few bugs in its Visitor caused by this pattern not being implemented  correctly)

## Implementing the DOM according to requirements

This is a somewhat simplified implementation: It does not employ composition sugar or macros to make reusing the same sub structures and vtable fields DRY, but it behaves very similar to the c++ reference impl here: https://gist.github.com/jdm/9900569

```rust
// <core>
#[lang="vtable"]
struct Vtable<T, Sized? Trait>;
extern "rust-intrinsic" { fn get_vtable<T, Sized? Trait>() -> Vtable<T, Trait>; }
#[lang="has_vtable"]
trait HasVtable<T, Sized? Trait> {}
// </core>

#[repr(fixed, inline_vtable)]
trait Node: Any where Self::V: HasVtable<Self, Node> {
    type V = Vtable<Self, Node>;
    vtable: V,

    parent: Rc<Node>,
    first_child: Rc<Node>,

    // Will be overridden in Element
    fn as_element<'a>(&'a self) -> Option<&'a Element> { None }
}

impl Node {
    // Non virtual method: Thanks to DST can be implemented directly on the trait object
    // (Its implementation still does a virtual call, its the
    // "sensible check" from the reference)
    fn as_text_node<'a>(&'a self) -> Option<&'a TextNode> {
        (self as &Any).downcast_ref::<TextNode>()
    }
}

struct TextNode {
    vtable: Vtable<TextNode, Node>,

    parent: Rc<Node>,
    first_child: Rc<Node>,
    ...
}
impl Node for TextNode {}

#[repr(fixed, inline_vtable)]
trait Element: Node where Self::V: HasVtable<Self, Element> {
    override type V = Vtable<Self, Element>;

    attrs: HashMap<String, String>;

    fn set_attribute(&self, key: &str, value: &str) {
        self.before_set_attr(key, value);
        //...update attrs...
        self.after_set_attr(key, value);
    }
    fn before_set_attr(&self, key: &str, value: &str);
    fn after_set_attr(&self, key: &str, value: &str);

    override fn as_element<'a>(&'a self) -> Option<&'a Element> { Some(self) }
}

struct HTMLImageElement {
    vtable: Vtable<Element, HTMLImageElement>,

    parent: Rc<Node>,
    first_child: Rc<Node>,

    attrs: HashMap<String, String>,

    ...
}

impl Element for HTMLImageElement {
    fn before_set_attr(&self, key: &str, value: &str) {
        if key == "src" {
            //..remove cached image with url `value`...
        }
        Element::default::before_set_attr(self, key, value);
    }
}

struct HTMLVideoElement {
    vtable: Vtable<Element, HTMLVideoElement>,

    parent: Rc<Node>,
    first_child: Rc<Node>,

    attrs: HashMap<String, String>,

    cross_origin: bool,
    ...
}

impl Element for HTMLVideoElement {
    fn after_set_attr(&self, key: &str, value: &str) {
        if key == "crossOrigin" {
            self.cross_origin = value == "true";
        }
        Element::default::after_set_attr(self, key, value);
    }
}

fn process_any_element(element: &Element) {
    // ...
}

let videoElement: Rc<HTMLVideoElement> = ...;
process_any_element(&*videoElement);

let node: &Node = &*videoElement.first_child;
let element = node.as_element();
match node.as_element() {
    Some(element) => ...,
    None => {
        let text = node.as_text_node().unwrap();
        ...
    }
}
```

# Drawbacks

Its a relatively complicated design

# Alternatives

There are many alternatives, anyone reading this probably knows this at this point ;)

Specific to this RFC however: There are many details that could be solved differently due to its puzzled-together nature. The hard part here is finding something that integrates well with other parts of the RFC.

# Unresolved questions

There are many details that needs to be carefully though through. The biggest potential problems are:

- Making associated fields memory safe (preventing aliasing between each other and real fields)
- Making default item overriding coherent
