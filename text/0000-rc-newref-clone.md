- Feature Name: refcount_clone_new_ref
- Start Date: 2017-03-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC proposes the simple addition of a new_ref method to reference-counted pointer types (Rc and Arc) in the standard library synonymous to clone, with a more explicit meaning.

# Motivation
[motivation]: #motivation

Clone is one of the most loosely defined concept in rust. Clone implementations, as the standard libray puts it "may or may not be expensive". The meaning of Clone can be shallow or deep clones alike.

It is therefore required, when reading code that calls clone to know the exact type of the calling object, because the cloning a ```Vec<T>``` and cloning an ```Arc<Vec<T>>``` have very different semantic and performance implications.

This leads to recurrent misunderstandings when reading or reviewing code [1].
In many cases the code could be a lot easier to understand at a glance if reference counted pointers had a method with a more explicit name to create new reference.

This issue is not specific to clone. The author of this RFC believes that it is generally best to avoid using a high level and vaguely defined concept as the only way to name an operation. ```Clone::clone``` is a prime example because it is at the same time one of the most vague concept, can imply either a very cheap or a very costly operation, and can go from one to the other with simple changes that don't necessarily affect the overal semantic of the progam.

# Detailed design
[design]: #detailed-design

A method ```fn new_ref(&self) -> Self``` is added to ```Rc<T>``` and ```Arc<T>```.
The ```Clone::clone``` implementations for ```Rc<T>``` and ```Arc<T>``` are ust calls to the respective new_ref methods.



This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

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

# Footnotes
[1] TODO: WebRender example.
