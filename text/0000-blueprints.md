- Feature Name: `blueprints`
- Start Date: 2032-04-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)

# Summary
[summary]: #summary

Adding blueprint "functions" which will be directly used instead of called.

# Motivation
[motivation]: #motivation

When looking though rust code the other day, especially looking at all those `new` functions which just create the type without any other or just a few expressions with it, i wondered how this would look in assembly. So I wrote a little experiment in godbolt and saw that a new function described as below results in a function with a call:
```rust
struct Test(u32);

impl Test {
    pub fn new(val: u32) -> Self {
        Self(val)
    }
}

pub fn main() {
    let _ = Test::new(12);
}
```
([reference](https://godbolt.org/#g:!((g:!((g:!((h:codeEditor,i:(filename:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,selection:(endColumn:2,endLineNumber:7,positionColumn:2,positionLineNumber:7,selectionStartColumn:2,selectionStartLineNumber:7,startColumn:2,startLineNumber:7),source:'struct+Test(u32)%3B%0A%0Aimpl+Test+%7B%0A++++pub+fn+new(val:+u32)+-%3E+Self+%7B%0A++++++++Self(val)%0A++++%7D%0A%7D%0A%0Apub+fn+main()+%7B%0A++++let+_+%3D+Test::new(12)%3B%0A%7D'),l:'5',n:'0',o:'Rust+source+%231',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0'),(g:!((h:compiler,i:(compiler:r1680,deviceViewOpen:'1',filters:(b:'0',binary:'1',binaryObject:'1',commentOnly:'0',demangle:'0',directives:'0',execute:'1',intel:'0',libraryCode:'0',trim:'1'),flagsViewOpen:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,libs:!(),options:'',selection:(endColumn:1,endLineNumber:1,positionColumn:1,positionLineNumber:1,selectionStartColumn:1,selectionStartLineNumber:1,startColumn:1,startLineNumber:1),source:1),l:'5',n:'0',o:'+rustc+1.68.0+(Editor+%231)',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0')),l:'2',n:'0',o:'',t:'0')),version:4))

Using a call for this isn't really what we want, we more likely just help the user by automatically filling some fields up of which the user shouldn't worry about, for example with a `Vec::new()` call or limiting the user to access some fields in a struct but make it still possible to create the struct.

Some people who really want this small performance improvement might [add `#[inline]` above](https://godbolt.org/#g:!((g:!((g:!((h:codeEditor,i:(filename:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,selection:(endColumn:13,endLineNumber:4,positionColumn:13,positionLineNumber:4,selectionStartColumn:13,selectionStartLineNumber:4,startColumn:13,startLineNumber:4),source:'struct+Test(u32)%3B%0A%0Aimpl+Test+%7B%0A++++%23%5Binline%5D%0A++++pub+fn+new(val:+u32)+-%3E+Self+%7B%0A++++++++Self(val)%0A++++%7D%0A%7D%0A%0Apub+fn+main()+%7B%0A++++let+_+%3D+Test::new(12)%3B%0A%7D'),l:'5',n:'0',o:'Rust+source+%231',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0'),(g:!((h:compiler,i:(compiler:r1680,deviceViewOpen:'1',filters:(b:'0',binary:'1',binaryObject:'1',commentOnly:'0',demangle:'0',directives:'0',execute:'1',intel:'0',libraryCode:'0',trim:'1'),flagsViewOpen:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,libs:!(),options:'',selection:(endColumn:1,endLineNumber:1,positionColumn:1,positionLineNumber:1,selectionStartColumn:1,selectionStartLineNumber:1,startColumn:1,startLineNumber:1),source:1),l:'5',n:'0',o:'+rustc+1.68.0+(Editor+%231)',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0')),l:'2',n:'0',o:'',t:'0')),version:4) but as you can see it still results in a call. Only when I mark the function [with `#[inline(always)]`](https://godbolt.org/#g:!((g:!((g:!((h:codeEditor,i:(filename:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,selection:(endColumn:20,endLineNumber:4,positionColumn:20,positionLineNumber:4,selectionStartColumn:20,selectionStartLineNumber:4,startColumn:20,startLineNumber:4),source:'struct+Test(u32)%3B%0A%0Aimpl+Test+%7B%0A++++%23%5Binline(always)%5D%0A++++pub+fn+new(val:+u32)+-%3E+Self+%7B%0A++++++++Self(val)%0A++++%7D%0A%7D%0A%0Apub+fn+main()+%7B%0A++++let+_+%3D+Test::new(12)%3B%0A%7D'),l:'5',n:'0',o:'Rust+source+%231',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0'),(g:!((h:compiler,i:(compiler:r1680,deviceViewOpen:'1',filters:(b:'0',binary:'1',binaryObject:'1',commentOnly:'0',demangle:'0',directives:'0',execute:'1',intel:'0',libraryCode:'0',trim:'1'),flagsViewOpen:'1',fontScale:14,fontUsePx:'0',j:1,lang:rust,libs:!(),options:'',selection:(endColumn:1,endLineNumber:1,positionColumn:1,positionLineNumber:1,selectionStartColumn:1,selectionStartLineNumber:1,startColumn:1,startLineNumber:1),source:1),l:'5',n:'0',o:'+rustc+1.68.0+(Editor+%231)',t:'0')),k:50,l:'4',n:'0',o:'',s:0,t:'0')),l:'2',n:'0',o:'',t:'0')),version:4) i can finally get what I want for the user.

And all this confusion with where to put `#[inline]` or `#[inline(always)]` and when and how it makes a difference is, in my view, not really ideal.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When you make a constructor function for your struct or a simple one liner util, it should most of the time be better to save this call in assembly and just direclty "paste" the inner code. This can easily achieved by blueprints:
```rust
// src/my_struct.rs
pub struct MyStruct(u32, Vec<()>);

impl MyStruct {
    #[blueprint]
    pub fn new(val: u32) -> Self {
        Self(val, Vec::new())
    }
}

// src/main.rs
fn main() {
    let _ = MyStruct::new(12);
}
```
This will result in something similar to this:
```rust
// src/my_struct.rs
pub struct MyStruct(u32, Vec<()>);

// src/main.rs
fn main() {
    let _ = MyStruct(12, Vec::new());
}
```
As you can see the function doesn't really exist in the binary, but its directly pasted. 

Another cool thing you might see from here is that the generated code sets the fields of `MyStruct` from another scope although they are private. So this means that you can also use blueprints to guide the user and let him use otherwise private and unusable fields in a safe environment with **zero-cost in binary**.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

I want to change the example a bit to demonstrate how the generated code should really look like:
```rust
// src/my_struct.rs
pub struct MyStruct(u32, Vec<()>);

impl MyStruct {
    #[blueprint]
    pub fn new(val: u32) -> Self {
        Self(val, Vec::new())
    }

    #[blueprint]
    pub fn get_number(&self) -> u32 {
        self.0
    }
}

// src/main.rs
fn main() {
    let x = 10 + 2;
    let my_struct = MyStruct::new(x);
    let y = my_struct.get_number();
    assert_eq!(x, y);
}
```
First, lets focus more on the `new` function. Here we have two obstacles:
* We access private fields
* We get passed a variable

Now lets inspect how the generated code could look like:
```rust
// src/my_struct.rs
pub struct MyStruct(u32, Vec<()>);

// src/main.rs
fn main() {
    let x = 10 + 2;
    let my_struct = {
        let val = x;
    	{ // now in here we should have the scope/permissions of the location where the blueprint was located previously, thus we can access the private fields
            Self(val, Vec::new())
        }
    };
    // ...
}
```
As you see, we first get all input parameters set in variables and then introduce another block which then should hold the scope of the used blueprint to have the ability to access the private fields.

Ok now with that knowledge, lets look at the rest of the generated code:
```rust
// src/my_struct.rs
pub struct MyStruct(u32, Vec<()>);

// src/main.rs
fn main() {
    let x = 10 + 2;
    let my_struct = {
        let val = x;
    	{ // now in here we should have the scope/permissions of the location where the blueprint was located previously, thus we can access the private fields
            Self(val, Vec::new())
        }
    };
    let y = {
        let self = &my_struct;
        {
            self.0
        }
    };
    assert_eq!(x, y);
}
```
Again, we first get all the parameters set and then evaluate the inner of the blueprint in the right environment.

# Drawbacks
[drawbacks]: #drawbacks

There are already `#[inline]` and `#[inline(always)]` but they are not that straight forward for beginners to just simply say "I want this to be pasted!"

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Instead of marking a function with `#[blueprint]`, I've come up with two other possibilities:
#### `blueprint` instead of `fn`
We could define a completly new element in the compiler called a blueprint which use would look like this:
```rust
// ...
impl MyStruct {
    pub blueprint new(val: u32) -> Self {
        Self(val, Vec::new())
    }
    // ...
}
// ...
```
This has the advantage of not having to put another attribute above functions, but that is it.
The disadvantages are that its more complicated for compiler-devs and just feels weird.

#### `blueprint!` macro
This could also co-exist to a `#[blueprint]` attribute and would allow the user to say whether he wants to blueprint a function. With that, the user could also easily use blueprints with older libraries that don't use them yet.

Usage example:
```rust
fn square(num: i32) -> i32 {
    num * num
}

fn main() {
    let x = blueprint!(square(12));
}
```

# Prior art
[prior-art]: #prior-art


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Im not sure which of the styles mentioned in [rationale-and-alternatives] is the best fitting.

# Future possibilities
[future-possibilities]: #future-possibilities

I think this can also come in very handy with const functions and const functions in traits. Think about `Index` or `Deref` for example. Most of the time they just call a method like `get` on the own struct and then `unwrap` it. The perfect use case for blueprints!