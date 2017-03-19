- Feature Name: refcount_clone_new_ref
- Start Date: 2017-03-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC proposes the simple addition of a new_ref method to reference-counted pointer types (Rc, Arc and repsective weak pointer types) in the standard library synonymous to clone, with a more explicit terminology.

# Motivation
[motivation]: #motivation

Clone is one of the most vaguely defined concept in rust. Clone implementations can be shallow or deep clones which, as the [standard libray documentation](https://doc.rust-lang.org/std/clone/trait.Clone.html) puts it "may or may not be expensive".

It is therefore required, when reading code that calls clone to know the exact type of the calling object, because the cloning a ```Vec<T>``` and cloning an ```Arc<Vec<T>>``` have very different logic and performance implications, while their semantic is really mostly driven by the type of ```T```.

This leads to recurrent misunderstandings when reading or reviewing code (See the example section at the end of this document).
In many cases the code could be a lot easier to understand at a glance if reference counted pointers had a method with a more explicit name to create new reference.

This issue is not specific to clone. The author of this RFC believes that it is generally best to avoid using a high level and vaguely defined concept as the only way to name an operation. ```Clone::clone``` is a prime example because it is at the same time an intentionally vague concept, can imply either a very cheap or a very costly operation, and can go from one to the other with simple changes to the code that don't necessarily affect the overal semantic of the progam. This RFC therefore concentrates on the case of Clone and reference counted types in the standard library.

# Detailed design
[design]: #detailed-design

This RFC does not involve any compiler change (only minor additions to alloc/rc.rs and alloc/arc.rs).

The following steps apply to ```Rc<T>```, ```Arc<T>```, ```rc::Weak<T>```, ```arc::Weak<T>```.

A method ```fn new_ref(&self) -> Self``` is added to the pointer type, into which the code of the ```Clone::clone``` implementation is moved.
The ```Clone::clone``` implementations for the pointer type simply becomes a ```new_ref``` call.

The proposed change is simple enough that it may be even simpler to see in code directly than in english.
It is therefore implemented on the author's [new_ref branch](https://github.com/nical/rust/tree/new_ref):
 - [Addition of Arc::new_ref](https://github.com/nical/rust/commit/392e105b0dd3ffb44beb8cbf853f75493a5167b5).
 - [Addition of Rc::new_ref](https://github.com/nical/rust/commit/5903ed4aa3ddb825f8b9b3412b3240f07193b711).
 - [Addition of arc::Weak::new_ref](https://github.com/nical/rust/commit/6f72fe1e208d96917c806bb4895b7014c1bfe164).
 - [Addition of rc::Weak::new_ref](https://github.com/nical/rust/commit/ddbd2b5e7e42d6be11194abef4f5d12ec11aa41e).

Note that these commits are missing the proper stable/unstable annotations.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Note that this RFC proposes a more descriptive way to do express an operation, but does not prevent from continuing to use the more generic way (clone still works as expected and remains the generic way to .

It would make sense for ```new_ref``` to be the preferred way to create new reference-counted pointers out of existing one, over the still existing ```clone``` method, since it is more descriptive.

This is not a big change in the standard library, however, common code examples in the official documentation and tutorials should be changed if ```new_ref``` becomes the preferred way over ```clone```.

# Drawbacks
[drawbacks]: #drawbacks

Adding methods (like ```new_ref``` to pointer types such as ```Rc<T>``` can create conflicts with the methods of the pointee type T. This can be worked around by defining new_ref as ```fn new_ref(this: &Self) -> Self;``` and calling it as follows: ```let thing_bis = Rc::new_ref(&thing);```.

Very short names are quite popular in the rust community for common operations. This RFC proposes a slightly longer name than the existing one.

# Alternatives
[alternatives]: #alternatives

This RFC tries to remain as straightforard and simple as possible. The tention between deep and shallow clones could maybe also be avoided through the addition of finer grained ShallowClone and DeepClone traits. It is not clear tothe author, however, if the added genericity would be useful in practice.

As described in the drawbacks section, ```new_ref``` is a new method on the pointer type which can conflict with the method of the pointee type. Defining the method as ```fn new_ref(this: &Self) -> Self;``` solves the issue. The drawback of using this pattern is that new_ref becomes a lot less simple to call than clone (for example ```let thing_bis = Rc::new_ref(&thing);``` versus ```let thing_bis = thing.clone();```), and the risk is that clone remains the popular way of copying reference counted pointers due to being easier to use.
If a trait (let's call it ```NewRef```) is added for this purpose, a function working on NewRef types could be added so that the call sites could look like ```let thing_bis = new_ref(&thing);``` which is is lighter, although arguably not as convelinet as clone still, and would require importing the function with a ```use``` statement.

The impact of not accepting this proposal would be that a "paper-cut" annoyance in the standard library remains.

# Unresolved questions
[unresolved]: #unresolved-questions

Is there a better name than ```new_ref```? The documentation uses the term _pointer_ in many places, although _reference_-counting is also present. ```new_ptr``` would work as well, although _ptr_ seems to be most used in referrence to raw pointers. ```clone_ref``` seems like another sufficitently descriptive name.
Longer names such as ```new_reference``` or ```new_pointer``` are just as informative, although the extra length goes against the general convention of using short names for common constructs.
Othe names such as ```new_rc```, ```new_arc```, etc. could be considered.

# Generalization (open discussion)

The vagueness of Clone is not criticised, here. Clone, as defined and with its abstraction level, serves its purpose well as a trait for generic operations. The problem comes from _more-specific_ operations and types, which would benefit from being expressed with descriptive terminology, being _only_ usable through high level or abstract terms.

The author of this RFC believes that as in general, types (structures and enums) should provide methods using adequately descriptive names and, _in addition_ to these methods, implement the more abstract traits using these methods. This way, no compromise is made on the names of the functionality exposed by the type. This should of course be taken with a grain of salt. It really depends on how far apart the type and a trait are in terms of the genericity of the concept they embody or describe.

# Example
Here is an example of code where the similarity between ```Vec<T>::clone()``` and ```Arc<Vec<T>>::clone()``` is a recurrent source of confusion:

[WebRender](https://github.com/servo/webrender)'s [resource_cache.rs](https://github.com/servo/webrender/blob/e1ba6ff8146a0ba7a33bb9af6390b34f6b313b78/webrender/src/resource_cache.rs#L381) now stores CPU image data as an ```Arc<Vec<u8>>```. These were originally simple ```Vec<u8>```s which were cloned each time they were be sent to a separate thread in charge of communicating with the GPU. Images being potentially very large, these clones were quite expensive and we decided to use Arcs to avoid the copy. Since cloning an entire vector and creating a reference-counted reference are both exposed through ```Clone::clone()``` (and _only_ through clone), the places where the expensive clones happened read exactly the same now that they only increment an atomic reference count. Long after this change, reading ```image.data.clone()``` still rings an alarm when going through or reviewing this code because it _reads_ like an expensive operation (copy the entire image) even though it is a simple atomic increment. People still occasionally talk about fixing these already-fixed clones and reviewers themselves have gotten it wrong at times. If it was possible to write ```image.data.new_ref()```, this simple code would be a lot less misleading.
This is only a simple example which isn't specific to the type of problem WebRender is solving. There are many other pieces of code out there where ```.clone()``` could be replaced by a more descriptive verb for improved clarity.
