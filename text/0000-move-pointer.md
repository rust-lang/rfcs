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
 - This pointer will get the same lifetime rules as `&` and `&mut` (such that it cannot outlive the allocation)
 - but, it will allow the value to become invalid before the allocation does.
 - Deref coercions as per RFC #241 apply.
 - Variables of type `&move T` will only allow mutating `T` if they themselves are `mut`
- Add a new unary operation `&move <value>`. With a special case such that `&move |x| x` parses as a move closure, requiring parentheses to parse as an owned pointer to a closure.
 - This precedence can be implemented by ignoring the `move` when parsing unary operators if it is followed by a `|` or `||` token.
- Add a new operator trait `DerefMove` to allow smart pointer types to return owned pointers to their contained data. 
 - A type that implements `DerefMove` cannot implement `Drop` (as `DerefMove` provides equivalent functinality)

```rust
trait DerefMove: DerefMut
{
    /// Return an owned pointer to inner data
    fn deref_move(&mut self) -> &move Self::Target;
    /// Equivalent to `Drop::drop` except that the destructor for `Self::Target` is not called
    fn deallocate(&mut self);
}
```

When an owned pointer is dropped (without having been moved out of), the destructor for the contained data is called (unlike `&mut` pointers, which are just borrows). The backing memory for this pointer is not freed until a point after the `&move` is dropped (likely either at the end of the statement, or at the end of the owning block).

For example, the following code moves out of a `Box<T>` into a `&move T` and passes it to a function
```rust
fn takes_move(val: &move SomeStruct) {
    // ...
}
fn main() {
    let val = Box::new( SomeStruct::new() );
    takes_move( &move val );
    println!("Hello");
}
```
This becomes the following operations
```rust
fn main() {
    let val = Box::new( SomeStruct::new() );
    takes_move( DerefMove::deref_move(&mut val) );
    DerefMove::deallocate(&mut val);
    println!("Hello");
}
```


# Drawbacks
[drawbacks]: #drawbacks

- Adding a new pointer type to the language is a large change

# Alternatives
[alternatives]: #alternatives

- Previous discussions have used `&own` as the pointer type
 - This name is far closer to the actual nature of the pointer.
 - But, since `own` is not a reserved word, such a change would be breaking.

# Unresolved questions
[unresolved]: #unresolved-questions

- `IndexMove` trait to handle moving out of collection types in a similar way to `DerefMove`
- Should (can?) the `box` destructuring pattern be implemented using `DerefMove`?
- Potential interactions of what happens when a `&move` is stored.
 - If a `&move` is stored in the current scope, when is the original storage freed?
 - If a `&move` isn't stored, is the storage freed right then, or when it would have otherwise gone out of scope?


# Appendix: Implementations of `DerefMove`
```rust
impl<T: ?Sized> DerefMove for Box<T>
{
    fn deref_move(&mut self) -> &move Self::Target {
        unsafe {
            &move *(self.0)
        }
    }
    fn deallocate(&mut self) {
        unsafe {
            heap::deallocate(self.0, mem::size_of_val(&*self.0), mem::align_of_val(&*self.0));
        }
    }
}
```

