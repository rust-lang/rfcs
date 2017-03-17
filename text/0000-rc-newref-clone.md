- Feature Name: refcount_clone_new_ref
- Start Date: 2017-03-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC proposes the simple addition of a new_ref method to reference-counted pointer types (Rc and Arc) in the standard library synonymous to clone, with a more explicit meaning.

# Motivation
[motivation]: #motivation

Clone is one of the most loosely defined concept in rust. Clone implementations can be shallow or deep clones which, as the [standard libray documentation](https://doc.rust-lang.org/std/clone/trait.Clone.html) puts it "may or may not be expensive".

It is therefore required, when reading code that calls clone to know the exact type of the calling object, because the cloning a ```Vec<T>``` and cloning an ```Arc<Vec<T>>``` have very different semantic and performance implications.

This leads to recurrent misunderstandings when reading or reviewing code [1].
In many cases the code could be a lot easier to understand at a glance if reference counted pointers had a method with a more explicit name to create new reference.

This issue is not specific to clone. The author of this RFC believes that it is generally best to avoid using a high level and vaguely defined concept as the only way to name an operation. ```Clone::clone``` is a prime example because it is at the same time one of the most vague concept, can imply either a very cheap or a very costly operation, and can go from one to the other with simple changes that don't necessarily affect the overal semantic of the progam.

# Detailed design
[design]: #detailed-design

This RFC does not involve any compiler change (only a minor addition to alloc/rc.rs and alloc/arc.rs.

A method ```fn new_ref(&self) -> Self``` is added to ```Rc<T>``` and ```Arc<T>```.
The ```Clone::clone``` implementations for ```Rc<T>``` and ```Arc<T>``` simply call their respective ```new_ref``` methods.


# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Note that this RFC proposes a more descriptive way to do express an operation, but does not prevent from continuing to use the more generic way (clone still works as expected and remains the abstract way.

It would make sense for ```new_ref``` to be the preferred way to create new reference-counted pointers out of existing one, over the still existing ```clone``` method, since it is more descriptive.

This is not a big change in the standard library, however, common code examples in the official documentation and tutorials should be changed if ```new_ref``` becomes the preferred way over ```clone```.

# Drawbacks
[drawbacks]: #drawbacks

Very short names are quite popular in the rust community for common operations. This RFC proposes a slightly longer name than the existing one.

# Alternatives
[alternatives]: #alternatives

Finer grained traits could probably be added to express shallow and deep copies, another trait could also be added to express adding a reference to a reference-counted object. It could also be added later

The impact of not accepting this proposal would be that a paper-cut of the language remains.

# Unresolved questions
[unresolved]: #unresolved-questions

Is there a better name than ```new_ref```?

# Notes
[1] Here is an example of code where the similarity between ```Vec<T>::clone()``` and ```Arc<Vec<T>>::clone()``` is a recurrent source of confusion:

[WebRender](https://github.com/servo/webrender)'s [resource_cache.rs](https://github.com/servo/webrender/blob/e1ba6ff8146a0ba7a33bb9af6390b34f6b313b78/webrender/src/resource_cache.rs#L381) now stores CPU image data as an ```Arc<Vec<u8>>```. These were originally simple ```Vec<u8>``` which were cloned each time they were be sent to a separate thread in charge of communicating with the GPU. Images being potentially very large, these clones were quite expensive and we decided to use Arcs to avoid the copy. Since cloning an entire vector and creating a reference-counted reference are both exposed through ```Clone::clone()```, the places where the expensive clones happened read exactly the same now that they only increment an atomic reference count. Long after this change reading ```image.data.clone()``` still rings an alarm when reading or reviewing this code because it _reads_ like an expensive operation (copy the entire image) even though it is a simple atomic increment. People still occasionally talk about fixing these already-fixed copies and reviewers themselves get it wrong. If it was possible to write ```image.data.new_ref()```, this simple code would be a lot less misleading.
