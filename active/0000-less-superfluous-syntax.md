- Start Date: 2014-10-09 
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Consider making the syntax less superfluous to make it easier to read and write code.

# Motivation

Often used code patterns should be as concise as possible, 
because they are repeated so often that writing them may get tedious for the programmer.
Current syntax is slightly superfluous and 
could be made more concise by omitting and replacing certain characters and patterns.

# Detailed design

Below are examples of proposed changes to the syntax:

### Omit ":" from function parameters and generics
```rust
fn print_area<T: HasArea>(shape: T) {
    println!("This shape has an area of {}", shape.area());
}
```
```rust
fn print_area<T HasArea>(shape T) {
    println!("This shape has an area of {}", shape.area());
}
```

### Omit ":" from let
```rust
let x: int = 5;
```
```rust
let x int = 5;
```

### Omit ":" from struct fields
```rust
struct Point {
    x: int,
    y: int,
}
```
```rust
struct Point {
    x int,
    y int,
}
```

### Omit "->" from functions
```rust
fn add_one(x: int) -> int {
    x + 1
}
```
```rust
fn add_one(x: int) int {
    x + 1
}
```

### Omit "->" from closures
```rust
let add_one = |x: int| -> int { 1i + x };
```
```rust
let add_one = |x: int| int { 1i + x };
```

### Replace namespace qualifier "::" with some single character symbol like ":", ".", "~"
```rust
hello::print_hello();
```
```rust
hello.print_hello(); 
```


### Replace "=>" in match with ":"
```rust
match cmp(x, y) {
    Less => println!("less"),
    Greater => println!("greater"),
    Equal => println!("equal"),
}
```
```rust
match cmp(x, y) {
    Less: println!("less"),
    Greater: println!("greater"),
    Equal: println!("equal"),
}
```

    

# Drawbacks

Obvious drawback is that this would make almost all code backwards incompatible.


# Alternatives

--

# Unresolved questions

Unfortunately, if operator overloading is not acceptable, 
character "." as the namespace qualifier will conflict with struct field expression.
On the other hand, character ":" might conflict with something else like the match proposal below.
Third option could be the "~" character.
```rust
hello~print_hello(); 
```
