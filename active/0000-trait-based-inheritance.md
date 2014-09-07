- Start Date: 2014-08-31
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

A handful of new traits handling zero-cost coercions and RTTI are added to Rust with compiler
support that allow efficient implementation of single inheritance in an orthogonal way.

# Motivation

Supporting efficient, heterogeneous data structures such as the DOM or an AST
(e.g., in the Rust compiler). Precisely we need a form of code sharing which
satisfies the following constraints:

* cheap field access from internal methods;
* cheap dynamic dispatch of methods;
* cheap downcasting;
* thin pointers;
* sharing of fields and methods between definitions;
* safe, i.e., doesn't require a bunch of transmutes or other unsafe code to be usable.

Moreover, in comparison to other proposals for inheritance, the design should work
well with existing Rust features and follow Rust's philosophies:

* There should be no new ways to achieve the same behavior. For example, virtual calls
  are currently only used in trait objects and function pointers, and this seeks to keep
  that list constant.
* Almost everything added should be useful in a general context, not specific to the
  fairly small (though important) use case of single inheritance.
* Performance decisions should be explicit - two pieces of code that look like they have
  the same performance characteristics should have the same performance characteristics.
* The solution for single inheritance should be modular so that people who want only a
  subset of the traditional features don't have to pay for the rest. For example, if there
  is no need for downcasting, it should be possible to not store RTTI.

# Detailed design

This design is very modular, and so is described in terms of its several different pieces.
Following the description of all the parts, a full example of using these parts for single
inheritance is given. Each section is titled both with the specialization of the feature to
single inheritance and, in parentheses, the more general use case of the feature.

## Marking the parent class (partially specifying layout)
To support zero cost upcasting, it is important for the data stored in a superclass to come
before any of the subclass's data, so that the pointer to the subclass object is exactly the
same pointer as the pointer to the upcasted object. This allows not just upcasting a single
object, but also upcasting an array of objects, all without any special computation.

To achieve this, a `#[first_field]` attribute is introduced, which is applied to some field of
a struct. This forces the data in that field to be layed out first in the struct and generates the
appropriate `Extend` impl (see the casting section).

As an example:
```
struct Node {
    // some data common to all nodes
    source_loc: uint
}

struct TextNode<'a> {
    #[first_field]
    node: Node,
    text: &'a str
}

struct ElementNode<'a> {
    #[first_field]
    node: Node,
    children: Vec<&'a Node>
}

fn example() {
    let node1 = TextNode {
        node: Node {
            source_loc: 1
        },
        text: "node1"
    }
    let node2 = ElementNode {
        node: Node {
            source_loc: 2
        },
        children: vec![]
    }

    let parent = ElementNode {
        node: Node {
            source_loc: 0
        },
        // the casting is described in a different section, but because of
        // the first_field annotation, it compiles to a noop
        children: vec![cast(&node1), cast(&node2)]
    }
}
```

## Upcasting (zero cost conversions)
Upcasting is just one example of many cases of pairs of types in which one has the
exact same representation as another. To encapsulate this relationship, a new trait,
`Cast`, is added, with no functions. The bound `A: Cast<B>` represents the statement
that something of type `A` can be converted to something of type `B` safely with a
simple `transmute`. To actually use this ability, a function is added to the standard
library as follows:

```
fn cast<A: Cast<B>, B>(input: A) -> B {
    unsafe {
        mem::transmute(input)
    }
}
```

Note that this function is not part of the definition of `Cast`. This allows the weakening
of the coherence restrictions necessary for making `Cast` useful: all the compiler cares
about is whether there exists an implementation, not which one it should use. Instead, to ensure
the termination of the bounds checking, a simple rule is imposed: for a given parameterized
type F, determining whether there exists some implementation of `F<A, B, ..> : Cast<F<A2, B2, ..>>`
must simplify to determining a whether some bounds between the pairs of parameters (`A` and `A2`,
`B` and `B2`) hold.

For example, there would be the following impls:

```
// If we can convert elements of a pair, we can convert the whole pair
impl <A: Cast<A2>, B: Cast<B2>, A2, B2> Cast<(A2, B2)> for (A, B) {}

// Although it is fine for the values to be changed, changing the key
// type or the hasher type would require the entries to be reordered,
// so we cannot allow any changes
impl <K, V: Cast<V2>, V2, H> Cast<HashMap<K, V2, H>> for HashMap<K, V, H> {}
```

If no impl is specified for a given parameterized type, the identity impl (`A: Cast<A>`) is assumed.

To utilize this system for upcasting, we introduce another trait, which encodes the subtype or
starts-with relationship. The bound `A: Entend<B>` represents that statement that, when viewed
behind a reference, something of type `A` can be converted to something of type `B`. Equivalently,
the representation of `A` starts with a representation of `B`. As suggested by the first formulation,
this is behind the implementation of `Cast` for references:

```
impl <'a, A: Extend<B>, B> Cast<&'a B> for &'a A {}

impl <'a, A: Extend<B>, B> Cast<&'a mut B> for &'a mut A {}
```

An `Extend` impl can be specified with a similar restriction to a `Cast` impl, but it can also be generated
by the use of the `first_field` attribute described previously. When a field marked `#[first_field]` is
visible, as follows, there is a bound `ChildType: Extend<ParentType>` in place:

```
struct ChildType {
    #[first_field]
    parent: ParentType,
    // Other fields
    foo: int,
    bar: bool
}
```

Note that the restriction that the first field be in scope is necessary to not break visibility boundaries.

To make this feature even more useful in contexts other than inheritance, the following impls are also present:

```
//pseudocode
impl < m less-than n > Extend<[a, ..m]> for [a, ..n] {}

impl <T> Cast<[T, ..1]> for T {}
impl <T> Cast<T> for [T, ..1] {}

impl Cast<int> for uint {}
impl Cast<uint> for int {}
impl Cast<i8> for u8 {}
impl Cast<u8> for i8 {}
//...
```

Additionally, the bounds checker is allowed to use transitivity:

```
impl <A: Extend<B>, B: Extend<C>, C> Extend<C> for A {}
impl <A: Cast<B>, B: Cast<C>, C> Cast<C> for A {}
```

With these traits, it is simple to create a traditional class, with both data and methods:

```
struct NodeData {
    // some fields here
}

trait Node: Extend<NodeData> {
    // some methods here, probably with defaults
}

// If not trying to make an "abstract class"
impl Node for NodeData {}
```

Anything implementing `Node` will implement the requisite methods and start with the correct fields.

## Bundling methods with objects (thin pointers to dynamically sized types)
While the machinery developed so far allows the construction of a trait corresponding to a traditional
class, there is no way to make a traditional thin pointer to an object instantiating a class. Simply
boxing the trait is insufficient, as this produces a fat pointer.

To talk about dynamically sized types properly, we say that every DST `T` has a corresponding type
`Discrim(T)` which is stored alongside pointers to `T`. Therefore, `Discrim(Trait) = &'static TraitVTable`,
and `Discrim([T]) = uint`.

To deal with the issue of fat pointers, a new pair of types (a statically sized version and a dynamically
sized version) are added. `Bundle<U, T>`, for `T` fitting the "pattern" of the dynamically sized `U`, is
just a pair `(Discrim(U), T)`. The dynamically sized version, `Bundle<U>`, has the same representation,
but does not have the exact type `T` stored in the type. However, since `Discrim(U)` is stored at a known
location in the type, the compiler can use thin pointers to these bundles: `Discrim(Bundle<U>) = ()`.
To fit into the previously described casting framework, the following impls are present:

```
impl <U, T> Cast<Bundle<U>> for Bundle<U, T> {}
impl <U: Cast<U2>, T: Extend<T2>> Cast<Bundle<U2, T2>> for Bundle<U, T> {}
impl <U: Cast<U2>> Cast<Bundle<U2>> for Bundle<U> {}
```

This new type fixes the problems with fat pointers to objects, as one can write `Box<Bundle<Node>>` instead
of just `Box<Node>`.

## Downcasting (safe RTTI)
The only item left on the list of requirements for inheritance is the problem of downcasting. To deal with this,
yet another trait is introduced, `Typed`. This trait has two properties that make it unique:

* Everything implements `Typed`. However, this is not "obvious" to the type system - although all *concrete* types
  implement `Typed`, it cannot be inferred that a generic type variable implements `Typed`.
* Instead of having methods, the "virtual table" for `Typed` is type information. This type information should be
  sufficient to determine whether the base type implements any given trait.

To use this type information, three functions are exposed that implement downcasting:

```
fn is_instance<A: Cast<B>, B: Typed>(value: &B) -> bool {
    value.get_type_info().matches::<A>()
}

fn downcast<A: Cast<B>, B: Typed>(value: B) -> Result<A, B> {
    if is_instance::<A, B>(&value) {
        unsafe {
            Ok(mem::transmute(value))
        }
    } else {
        Err(value)
    }
}

fn downcast_copy<A: Cast<B>, B: Copy + Typed>(value: B) -> Option<A> {
    if is_instance::<A, B>(&value) {
        unsafe {
            Some(mem::transmute(value))
        }
    } else {
        None
    }
}
```

## Summary example

```
struct NodeData {
    source_loc: uint
}

type NodeBox<'a> = Box<Bundle<Node + Typed + 'a>>;

trait Node: Extend<NodeData> {
    fn children<'a>(&'a self) -> Vec<NodeBox<'a>>
}


struct TextNode<'a> {
    #[first_field]
    node: NodeData,
    text: &'a str
}

type TextNodeBox<'a> = Box<Bundle<Node + Typed + 'a, TextNode<'a>>>;

impl <'a> Node for TextNode<'a> {
    fn children<'b>(&self) -> Vec<NodeBox<'b>> {
        vec![]
    }
}


struct ElementNodeData<'a> {
    #[first_field]
    node: NodeData,
    children: Vec<NodeBox<'a>>
}

type ElementNodeBox<'a> = Box<Bundle<ElementNode<'a> + Typed + 'a>>;

trait ElementNode<'a>: Node + Extend<ElementNodeData<'a>> {
    fn element_type(&self) -> String;
}

impl <'a> Node for ElementNodeData<'a> {
    fn children<'b>(&'b self) -> Vec<NodeBox<'b>> {
        self.children.clone()
    }
}


struct ImgElement<'a> {
    #[first_field]
    element: ElementNodeData<'a>,
    width: uint,
    hieght: uint,
    src: &'a str
}

impl <'a> Node for ImgElement<'a> {
    fn children<'b>(&'b self) -> Vec<NodeBox<'b>> {
        self.element.children()
    }
}

impl <'a> ElementNode<'a> for ImgElement<'a> {
    fn element_type(&self) -> String {
        "img".to_string()
    }
}


fn dump<'a>(node: NodeBox<'a>) {
    if let Ok(text_node): Option<&TextNodeBox<'a>> = downcast_copy(node) {
        println!("Found text node: {}", text_node.text);
    } else if let Ok(element_node): Option<&ElementNodeBox<'a>> = downcast_copy(node) {
        println!("Found element node: {}", element_node.element_type());
    } else {
        println!("Found unknown node!");
    }

    for child in node.children().iter() {
        dump(child);
    }
}
```

# Drawbacks

* This results in verbose declarations of classes. This is unfortunate, but could probably be
  fixed with a macro. Note that this verbosity does give significant control absent in other
  proposals for inheritance - it is possible to choose on a case by case basis between static
  and virtual dispatch of methods, between storing and not storing RTTI, and between fat and
  thin pointers.
* This proposal is very large. This is somewhat a side effect of trying to make everything
  useful for even Rust code that doesn't touch inheritance.
* The RTTI may not be as efficient as it could be. This section is the least well thought out
  section of the whole proposal, and may require O(n) processing of type information. However,
  this type information is optional, and it may turn out to be good practice not to use it.
* The user is forced to think about how classes are actually implemented instead of just writing
  classes or virtual structs. Depending on the viewpoint, this could actually be an advantage, as
  it makes the user decide which parts of inheritance they really want. Additionally, this burden
  would be significantly lessened by having a macro for creating classes.
* This proposal may be difficult to learn. Again, this would be improved by having a macro for
  creating classes.

# Alternatives

## Other inheritance proposals
* Virtual structs (#5). This is probably the simplest inheritance proposal out there, as it simply
  adds the traditional class structure to Rust. However, because of the ad hoc way it deals with
  inheritance, it duplicates functionality present in other parts of Rust and doesn't allow for much
  configurability. For example, they add a new way to perform virtual calls and restrict the user to
  using thin pointers, RTTI, and virtual calls in all situations, even when they might not want all of
  those features. Virtual structs are a perfect solution for a common problem, but don't work well outside
  of that problem.
* Fat objects (#9). The `Bundle` object described here is exactly what was proposed in this RFC. While
  this RFC briefly touched on inheritance, that wasn't the focus, and so only a sketch was given as to
  how fat objects would fit into an inheritance framework. As such, there is no real comparison to be
  made - what was proposed there is a subset of what is proposed here.
* Extending enums (#11). This seems to be similar to #142, having roughly the same limitations.
* Efficient single inheritance (#142). This is a major change to the language, including trying to unify
  structs and enums, but the part of it important for comparing inheritance is that it introduces the idea
  of struct variants, which add data and methods to the base struct and can override certain methods. To
  implement this, it adds a pointer to a virtual table at the front of each such struct. This has the
  interesting property that inheritance (creating struct variants) is closed - it is not allowed to create
  new variants outside of the module in which the base struct is defined. While this proposal has some very
  nice attributes (like very efficient downcasting), it still fails to achieve many of the goals set out above.
  It introduces a new way of performing virtual calls (through structs) that is completely separate from the
  existing mechanisms. Worse, it makes it no longer obvious whether calling a method on a struct will result in
  virtual dispatch or static dispatch, as this requires looking at the definition to see if it is declared virtual.
  Additionally, although there is significantly more complexity introduced, surprisingly little functionality is
  added - it just adds the nesting of structs and enums and single inheritance. Although these features work very
  well together, there is very little functional change, as the nesting is merely a (much) prettier way of doing
  something already possible. Despite these limitations, this proposal does some things much better, including having
  efficient downcasting, a clean syntax, and closed inheritance (though it is as of yet unclear how good the last is).

## Modifications
* Instead of using a `#[first_field]` attribute, one could write `struct Child: Parent` and have the compiler automatically
  add a `super` field that is placed first.
* Instead of using a `#[first_field]` attribute, the compiler could just detect a special field name like `super` and declare
  that to be the first field.
* By adding versions of `Extend` and `Cast` that are parameterized on a lifetime, the casting mechanism could be extended to
  support mass borrowing and slicing. For example, `Box<T>` could implement `BorrowedCast<'a, &'a T>`, signifying that, when
  viewed within a lifetime `'a`, it can be converted to a reference of lifetime `'a`. Similarly, `Vec<T>` would implement
  `BorrowedExtend<'a, &'a [T]>`, meaning that it starts with a slice, but can only be viewed as a slice when looked at with
  a correct lifetime. Then the implementation of `Cast` for `&` would be as follows:

  ```
  impl <'a, A: BorrowedExtend<'a, B>, B> Cast<&'a B> for &'a A {}
  ```

  This would allow, for example, casting `&'a Vec<Box<T>>` to `&'a [&'a T]`.
* Compiler support for RTTI could be dropped, forcing the user to do this manually. As an example, the `Node` trait in the example
  above would have new methods `as_text_node<'a>(&'a self) -> Option<&'a TextNode<'a>>` and `as_element_node`, and `ElementNode`
  would have `as_img_node`. This would cause huge amounts of boilerplate, but would be conceptually simpler and would be possibly
  more efficient (constant time).

## Bikeshedding
* Rename `#[first_field]` to `#[super]` or `#[extend]`.
* Rename `Cast` to `Coerce`, `Coercible`, `Transmute`, `SameRepr`, `Convert`, or `Upcast`.
* Rename `Extend` to `HasPrefix` or `StartsWith`.
* Rename `Bundle` to `Fat`, `Thin`, or `BehindPointer`.
* Rename `Typed` to `HasType`, `Typable`, or `RTTI`.

# Unresolved questions

* Is there a better way to deal with downcasting?
* Is downcasting ever a good idea? What use cases does it have?
* Are there better names?
* Is there a way to limit inheritance, both in terms of overriding methods and in terms of where a "class" can be overridden?
* Are those limits desirable?
