- Start Date: 2014-09-13
- RFC PR:
- Rust Issue:

# Summary

Structs are given non-subtyping inheritance of other structs. Traits can also inherit from structs, which works analogously to traits inheriting traits: an implementor of such a trait must also inherit from its structs. Match statements are extended to work on trait objects. Combined with fat objects, this enables traditional inheritance-based designs using (mostly-)existing orthogonal features.

# Motivation

Rust needs inheritance to implement data structures like the DOM or ASTs efficiently. We have the following constraints:

* Cheap access to shared fields (through known constant offsets)
* Cheap dynamic dispatch to shared interfaces (through thin pointers and vtables)
* Cheap downcasting
* Safety

This proposal also has these goals:

* Let existing features fulfil their purpose in more situations rather than adding new features
* When new features are necessary, make them orthogonal to existing features and generally useful
* Keep Rust a systems language - give precise control over performance characteristics

# Detailed design

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

Traditional implementations of inheritance are used to achieve several results. This design applies an existing or new feature to each of those uses.

## Struct Layout

Rust struct layout is currently undefined by default, with `#[repr(C)]` as an alternative for FFI. Another reason to specify struct layout is for cheap access to fields shared between types. This RFC proposes an inheritance-like syntax:

```Rust
struct A {
	x: i32,
	y: i32,
}

struct B: A {
	a: u32,
	b: u32,
}
```

This is identical to giving `A` a field of type `B`, except that its fields are directly accessible and are laid out at the beginning of the struct. There is no subtyping relation.

## Traits Inheriting Structs

Traits can currently contain associated methods, which means implementors of the trait must implement those methods. Another property traits need to specify is that their implementors have some particular fields at a fixed offset. This is where the struct layout makes a difference for the type system. To continue the example above:

```Rust
trait T: A {
	fn use_x(&self) {
		// use self.x
	}
}

impl T for B {}
```

This means any implementor of `T` must begin with the fields of `A`, which can be accessed either in monomorphized functions or through trait objects at a fixed offset.

## Casting and Vtable Layout

Trait objects need to be upcast to parent trait objects or downcast to child trait objects or concrete implementors. Upcasting is business as usual: `some_struct as &SomeTrait`, or `some_trait_object as &SuperTrait`. For downcasting, the necessary information is kept alongside the object's vtable (like associated statics), and that can be used in a `match`:

```Rust
trait A {}
trait B: A {}
trait C: A {}

struct D { ... }
impl B for D {}

struct E { ... }
impl C for E {}

let a: &A = ...;
match a {
	d as &D => ..., // matches a concrete D
	b as &B => ..., // matches any other implementor of B
	c as &C => ..., // matches anything implementing C, such as an E

	// one of these would be required for public/cross-crate traits with potentially-unknown implementors
	a as &A => ...,
	_ => ..., 
}
```

The implementation of downcasting is much simpler than for C++, which has to deal with multiple and virtual inheritance. Conceptually, the inheritance chain needs to be followed from the target through its parents until the source is found or the chain ends. As an optimization, any redundancy can be removed because all the checks are in a single `match`.

Each vtable needs to be prefixed by the vtables of its parent traits. When casting between trait objects (`some_trait_object as &SuperTrait` or, in a match, `some_trait_object as &SubTrait`) this gives a simple offset for the vptr, which is zero in single inheritance hierarchies. This is important for fat objects (see below).

## Fat Objects

Trait objects (and other DSTs) normally store their vptr (or size) alongside the object pointer (a "fat pointer"), but it's often better to store the vptr alongside the object itself (a "fat object" with a "thin pointer"). [RFC PR #9](https://github.com/rust-lang/rfcs/pull/9) describes how to express fat objects:

> For any `T`, `U` and `v` such that `Fat(T as U) = v`, it is possible to create a "fat object" which carries the extra word along with it: `(v,T)`.  For traits, this layout is the same as that of a C++ object that starts with its vtable.
> 
> Syntactically, this is expressed as if it were a generic struct that can take either one or two type parameters:
> 
> * `Fat<U,T>` denotes a fat concrete object type.  In memory it is laid out as if it were the tuple `(v,T)` for a specific `T` and `U` where `Fat(T as U) = v`.  `T` must be a concrete type, and `U` must be an existential variant of that type.  So either `T` is `[Elem, ..n]` and `U` is `[Elem]`, or `U` is a trait implemented by the concrete type `T`.  `Fat<U,T>` is not dynamically sized, so it can be used anywhere a normal statically sized type can--as a variable or parameter, inside an array, etc.  It implements `Deref<T>` and `DerefMut<T>`, which obtain borrowed pointers to the plain `T` object.  It also has a method `unwrap() -> T` that discards the extra word and leaves a plain `T`.  There is no automatic implementation added such that `Fat<U,T>` implements trait `U`, but auto-deref should allow you to call methods of any trait implemented by `T` (including but not limited to `U`).
> * `Fat<U>`  is a dynamically sized fat existential type.  `U` must be a dynamically sized type.  Just like a plain `U` is a stand-in for any `T` where `T as U`, `Fat<U>` stands in for any `Fat<U,T>` where `T as U`.  Since it is dynamically sized, `Fat<U>` is only usable in the places where other DSTs can be used.  But unlike other DSTs, pointers to `Fat<U>` are plain old thin pointers.  When the object is used, the required extra word can be retrieved from its place right before the object itself.
> 
> A fat object can only be fat for one trait, no matter how many traits the type actually implements.
> 
> An intrinsic function `fat` is added to `std` to create fat objects:
> 
> `fn fat<unsized U,T>(t:T) -> Fat<U,T>`
> 
> It is allowed when `U` is a trait implemented by `T`, and when `T` is a fixed length array and `U` is the corresponding unknown-length array of the same type.
> 
> The same pointer conversion rules laid out in the DST proposal between `T` and `U` also apply to `Fat<U,T>` and `Fat<U>`.  The only difference is that the pointers to `Fat<U>` are still thin pointers, despite `Fat<U>` being dynamically sized.  So a `&Fat<U,T>` can be automatically converted to a `&Fat<U>`, and these pointers have the exact same representation.
> 
> For structs that are themselves dynamically sized because they end with a DST, "fatness" propagates up the chain.  Using the `RcData` example from the DST propsal, `RcData<Fat<U>>` would be considered a fat DST, and therefore the pointer inside a `Rc<Fat<U>>` would be a thin pointer.

Casting fat objects between elements of a single inheritance hierarchy is possible using `some_fat_trait_object as &Fat<SuperOrSubTrait>`, because the vptr does not change. In a more complicated hierarchy, only fat pointers are usable (which is the only way to downcast anyway).

## JDM's Example

Here is [JDM's DOM example](https://gist.github.com/jdm/9900569) using this proposal:

```Rust
trait Node: NodeData {
}

struct NodeData {
	parent: Rc<Fat<Node>>,
	first_child: Rc<Fat<Node>>,
}

struct TextNode: NodeData {
}

impl Node for TextNode {}

trait Element: Node + ElementData {
	fn before_set_attr(&mut self, key: &str, val: &str) { ... }
	fn after_set_attr(&mut self, key: &str, val: &str) { ... }

	// this way, set_attribute is monomorphized
	// taking e.g. &mut Fat<self> should allow dynamic dispatch without monomorphization
	fn set_attribute(&mut self, key: &str, value: &str) {
		self.before_set_attr(key, value);
		// update
		self.after_set_attr(key, value);
	}
}

struct ElementData: NodeData {
	attrs: HashMap<String, String>,
}

struct HTMLImageElement: ElementData {
}

impl Node for HTMLImageElement {}

impl Element for HTMLImageElement {
	fn before_set_attr(&mut self, key: &str, val: &str) {
		if key == "src" {
			// remove cached image
		}
		Element::before_set_attr(key, value);
	}
}

struct HTMLVideoElement: ElementData {
	cross_origin: bool,
}

impl Node for HTMLVideoElement {}

impl Element for HTMLVideoElement {
	fn after_set_attr(&mut self, key: &str, val: &str) {
		if key == "crossOrigin" {
			self.cross_origin = value == "true";
		}
		Element::after_set_attr(key, value);
	}
}

// or &JS<Fat<Element>>, etc.
fn process_any_element(element: &Fat<Element>) { ... }

let video_element: Rc<Fat<HTMLVideoElement>> = box (Rc) fat(HTMLVideoElement { ... });
process_any_element(&*video_element);

let node = video_element.first_child.clone();
match &**node { // turn Rc<Fat<Node>> into &Node
	element as &Element => ...,
	text as &TextNode => ...,
	_ => ...,
}
```

# Drawbacks

* This RFC adds complexity to struct definitions and layout, trait inheritance, pattern matching, and DSTs.
* Defining a class hierarchy this way is more verbose than something like C++, because each `Trait` also needs a `TraitData`. However, this does add flexibility at the same time.
* The interaction between casting fat objects and inheritance is fragile, and only allows such casting for single inheritance chains.

# Alternatives

* Implement another of the inheritance RFCs. These tend to add new features that overlap existing ones, conflate existing features with new ones, leave existing features out where they would be useful in relation to inheritance, or create extra complexity.
* Allow multiple inheritance for generic use of traits (not for trait objects):
  ```Rust
  trait T1: A {}
  trait T2: B {}

  struct A { a: i32 }
  struct B { b: i32 }

  struct C: A + B {}

  fn use_multiple_inheritance<T: T1 + T2>(t: &T) {
  	// use t.a and t.b
  }

  // this cannot be called with a &C:
  fn use_t1(t: &T1) { ... }
  ```
  I'm pretty sure this is a backwards compatible addition.
* Associated fields: Don't specify struct layout, but let an `impl T for A` where T includes a struct `B` specify which member (possibly unnamed) of `A` fulfills that contract:
  ```Rust
  struct A {
  	struct_b: B // or ..B
  }
  
  struct B {
  	x: i32
  }
  
  trait T {
  	trait_b: B // or B trait_b
  }
  
  impl T for A {
  	use struct_b as trait_b; // or leave it out for unamed fields
  }
  ```
  This complicates the syntax for associated items and struct definitions, and limits cheap field access to monomorphized functions. This might be an acceptable tradeoff if accessing members through trait objects doesn't need to be fast (it could be done through virtual accessors or associated static pointers-to-members).
* Don't modify DST for fat objects, and implement thin pointers as a library type instead, like [krdln's gist](https://gist.github.com/krdln/08764f70a1e5aeda2338). This breaks smart pointers to fat objects, because there's no way to tell the smart pointer that its referent already has its vptr.
* `container_of`: One example of this is linked lists in the Linux kernel. Instead of knowing the offset of common fields given a pointer, use pointers to the common fields and access the containing struct by offsetting the pointer. Looks like this in C:
  ```C
  struct my_struct {
  	const char *name;
  	struct list_node list;
  };
  
  struct list_node *current_node = ...;
  struct my_struct *element = container_of(current_node, struct_my_struct, list);
  ```
  This is unsafe and making it safe with thin pointers would probably be tricky.

# Unresolved questions

* Syntax for turning `Fat<T>` into `&T`: reborrowing a fat object isn't terrible, but there could be a better way.
* Syntax for matching on trait objects: I like the `a as &A` syntax because it works when casting to other trait objects and to the actual implementors of the trait, but maybe someone has a better idea?
* Syntax for fat objects: the `Fat<T, U>` and `Fat<T>` thing doesn't quite match existing generics syntax, and it's unclear that it's built in (although types affecting their containing types is not unprecedented). On the other hand, another syntax might be more complicated or confusing.
* There are a few nice backwards compatible library additions, like `trait Extends<T>: T {}` or `fn as<T, U>(t: &T) -> Option<&U>` that may or may not be worth having.
