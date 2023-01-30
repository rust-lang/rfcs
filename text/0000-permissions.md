- Feature Name: `permissions`
- Start Date: 2022-01-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)

# Summary
[summary]: #summary

Set permissions for functions and add permissioned borrows.

# Motivation
[motivation]: #motivation

So I've been latetly writing a lot of rust code where I need a struct mutable borrowed and stored paralelly to other borrows of the same data.
They really don't interop or block each other, they are two iterators nested and one needs the whole type to call a function on it (immutable) and get that data.
The other iterator needs a mutable borrow of an underlying `HashMap` in that same struct to convert the key to a mutable ref to the value.
But these two iterators nested wouldn't work, because they would both reference to the same data.
To resolve the error: ```cannot borrow `*app` as mutable more than once at a time
second mutable borrow occurs here```, I've just be using unsafe pointer magic..:
```rust
// ..with a safety comment above
let ptr: *mut G = &mut *app;
let inner = Self::new(&*ptr);
```
While I wrote this, the idea of a langauge with permissions came up to me.

> The explanation of how it should work follows in the next chapter.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

So, a permission is looking like a life-time specifier.
Imagine we have a struct for a person:
```rust
pub struct Person {
  name: String,
  age: u8,
}
```
And now we want a function for `Person` which lets us age him/her by one year and another one for renaming him/her.
```rust
impl Person {
  pub fn aging(&mut self) {
    self.age += 1;
  }
  
  pub fn rename(&mut self, new_name: String) -> String {
    std::mem::replace(&mut self.name, new_name)
  }
}
```
Not too fancy, right? But we now want a `Department` for changing his name and a `Calendar` to age a person by one year.
```rust
pub struct Department<'p>(&'p mut Person); // ideally we want more than one person, but "simplicity"

impl Department<'_> {
  pub fn rename(&self, new_name: String) -> String {
    self.person.rename(new_name)
  }
}

pub struct Calendar<'p>(&'p mut Person); // ideally we want more than one person, but "simplicity"

impl Calendar<'_> {
  pub fn next_year(&self) {
    self.person.aging();
  }
}
```
Still simple, right? So let us create some life 😊
```rust
fn main() {
  let mut person = Person {
    name: "Mike",
    age: 0,
  };
  
  let department = Department(&mut person);
  let calendar = Calendar(&mut person); // ERROR: cannot borrow `*person` as mutable more than once at a time second mutable borrow occurs here
  
  calendar.next_year();
  calendar.next_year();
  calendar.next_year();
  department.rename("Jonathan");
  calendar.next_year();
}
```
Dang it! But with permissions, we can easily fix this. So let us change some things in our `impl` of `Person`:
```rust
impl Person {
  pub fn 'Age aging(&mut self) { // this function needs the permission `'Age`
    self.age += 1;
  }
  
  pub fn 'Name rename(&mut self, new_name: String) -> String { // this function needs the permission `'Name`
    std::mem::replace(&mut self.name, new_name)
  }
}
```
Now let us change the borrow for `Department` and `Calendar`:
```rust
pub struct Department<'p>(&'p 'Name mut Person); // the borrow has the permission `'Name`
pub struct Calendar<'p>(&'p 'Age mut Person); // the borrow has the permission `'Age`
```
And now let's see our main code:
```rust
fn main() {
  let mut person = Person {
    name: "Mike",
    age: 0,
  };
  
  let department = Department(&mut person); // implicit permission
  let calendar = Calendar(&'Age mut person); // explicit permission
  
  calendar.next_year();
  calendar.next_year();
  calendar.next_year();
  department.rename("Jonathan");
  calendar.next_year();
}
```
And TADA: our code compiles and works 🥳.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The compiler should know what the permissions allow of data to use.
For example, when two functions need the same data mutably and normally it wouldn't compile, they shouldn't just add two different permissions to evade it:
```rust
impl Hack {
  pub fn 'HackA hack_a(&mut self) {
    self.one_data./*...*/
  }
  
  pub fn 'HackB hack_b(&mut self) {
    self.one_data./*...*/
  }
}
```
Instead, the compiler should notify that the permissions intercept with each other and they should create a sub-permission for both:
```rust
impl Hack {
  pub fn 'OneData + 'HackA hack_a(&mut self)
  where
    self.one_data: 'OneData // I'm unsure about this syntax
  {
    self.one_data./*...*/
  }
  
  pub fn 'OneData + 'HackB hack_b(&mut self)
  where
    self.one_data: 'OneData
  {
    self.one_data./*...*/
  }
}
```

# Drawbacks
[drawbacks]: #drawbacks

It may be confusing especially for newbies to differentiate between lifetimes and permissions. Maybe something else than `'`?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I think this is one of the cleanest options for these problems. Other devs may be changing their whole code structure for that or use `unsafe`, but I like the idea of permissions.
It maybe "could" be done with macros, but then it would also generate unsafe code and wont be 100% safe.. but the simplest impl of permissions may be possible to implement via them.

# Prior art
[prior-art]: #prior-art

I don't know any language who has done something like this before.
The idea came to me by minecraft server development and there its used for giving players the permission to execute commands, so it may be similar in the smallest way possible 😅.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is there a clearer way to solve this problem?
* Is there a cleaner syntax for this? (especially for the `where self.one_data: 'OneData`.

# Future possibilities
[future-possibilities]: #future-possibilities

I think this could change rust to a even safier programming language as its a extention to already existing methods like borrowing or lifetimes.
It would remove lots of `unsafe` blocks without having to reinvent and rething about the whole program.