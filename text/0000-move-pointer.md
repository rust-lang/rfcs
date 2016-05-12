- Feature Name: `move_pointer`
- Start Date: 2016-05-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a new pointer type `&move` that logically owns the pointed-to data, but does not control the backing memory. Also introduces the DerefMove trait to provide access to such a pointer.

# Motivation
[motivation]: #motivation

This provides an elegant solution to passing DSTs by value, allowing `Box<FnOnce>` to work. It also will allow other usecases where trait methods should take self by "value".

# Detailed design
[design]: #detailed-design

- Add a new pointer type `&move T`
- Add a new unary operation `&move <value>`. With a special case such that `&move |x| x` parses as a move closure, requiring parentheses to parse as an owned pointer to a closure.
- Add a new operator trait `DerefMove` to allow smart pointer types to return owned pointers to their contained data. 
 - ```rust
trait DerefMove: DerefMut
{
    /// Return an owned pointer to inner data
    fn deref_move(&mut self) -> &move Self::Target;
    /// Drop self without calling destructor for Self::Target
    fn deallocate(self);
}
```


When an owned pointer is created to a variable (e.g. on the stack) the owner of the pointer takes responsability of calling the destructor on the pointed-to data (and can do any operation assuming that it has full ownership of the object). The original owner still controls the memory allocation used to hold the type, and will deallocate that memory in the same way as if the object had been passed by value.

# Drawbacks
[drawbacks]: #drawbacks

- Adding a new pointer type to the language is a large change

# Alternatives
[alternatives]: #alternatives

- Previous discussions have used `&own` as the pointer type
 - Since `own` is not a reserved word, such a change would be breaking.

# Unresolved questions
[unresolved]: #unresolved-questions

- `IndexMove` trait to handle moving out of collection types in a similar way to `DerefMove`
- Should (can?) the `box` destructuring pattern be implemented using `DerefMove`?

