- Start Date: 2014-06-24
- RFC PR #: 
- Rust Issue #: 

# Summary

Objects of type `T` should be implicitely convertible to `&T`.

# Motivation

 * There is an inconsistency between member functions and external functions.
 When you call a member function, the object is automatically borrowed when the function requires a `&self`.
 However when you pass an object to a function, the `&` must be explicit.

      struct Foo;
      impl Foo {
       fn member_ref(&self) {}
    	  fn member_val(self) {}
      }
      fn extern_ref(&Foo) {}
      fn extern_val(Foo) {}

      let a = Foo;
      let b = Foo;

      a.member_ref();  // ok, automatically borrowed
      a.member_val();  // ok
      extern_ref(&b);  // different syntax, why is the '&' needed in this scenario?
      extern_val(b);   // ok

 * You need to differenciate between local variables and parameters only in some cases.
 I think that you should either always differenciate between references and values, or never. But not *sometimes*.

      fn bar(a: &Vec<int>) {}

      fn foo(a: &Vec<int>) {
        let b: Vec<int> = vec!(2, 4, 73, -7);

        // you use a and b the same way, they are indifferentiable:
        for elem in a.iter() {}
        for elem in b.iter() {}
        a.get(0);
        b.get(0);

        // ...except when you pass them to functions:
        bar(a);
        bar(&b);  // different syntax
      }

 * The `hashmap.find(&5)` syntax is ugly and counter-intuitive.

 * When a function needs an `int` parameter for example, people will simply add `value: int` to the parameters list.
 This implies that the caller will need to make a clone of an existing object, even though the parameter is actually not consumed by the function.
 This is ok for `int`s because they are free to clone, but it is not possible when you use templates.

 This leads to inconsistencies in APIs, like the one below. The `get` function expects a value while
 the `find` function expects a reference, even though both functions are very similar.

      let index = 5;

      let a: Vec<int> = ...something...
      a.get(index)     // the function expects a value

      let b: HashMap<uint, int> = ...something...
      b.find(&index)   // the function expects a reference

      // I prefer the syntax of "get", but "find" is more correct because you shouldn't consume the index



# Detailed design

The main modification is that objects of type `T` should be implicitely convertible to `&T`.

For both local variables:

    let x: int = 5;
    let y: &int = x;    // currently not ok, but would become ok

And function parameters:

    fn foo(val: &int) {}
    foo(5);   // currently not ok, but would become ok

This will encourage people to add an `&` before their parameter type whenever they only need to read the value instead of consuming it.
For example the `Vec::get` function would have the `fn get(&self, &uint)` definition instead.

Note that this only concerns `&T`. The `&mut` syntax should still be mandatory when passing an object by mutable reference, because it explicitly says that the variable will be modified.

The consequence of this and second proposed change is that when a `&T` parameter is requested and `T` is a base type (integers, floats, etc.), then the compiler should be allowed to optimize this by sending the value itself instead of a pointer to the value. The purpose of this is to avoid the performance hit of passing a `&uint` instead of an `uint`.


# Drawbacks

 * People may argue that you won't be able to tell whether you are moving a value or passing a reference.
 For example if you write `foo(a)`, you don't know whether you move `a` or simply pass `&a` to the function without looking at `foo`'s definition. But is this really a problem? All you risk is a compilation error, there are no safety issue.

 Also this problem is already more or less here today, since it depends whether or not `a` is already a reference itself.

 * This would turn Rust into a higher-level language than it is today, so maybe there will be some ideology issues. But I feel that any change that could bring the language to a higher-level without any performance hit is a good change.


# Alternatives

Keep the current syntax, which I feel is both inconsistent and a bit annoying to code with.


# Unresolved questions

 * Allowing the compiler to optimize `&int` as if it was `int` means that `let mut inputMut: &mut int = std::mem::transmute(input); (*inputMut) = 5;` will not behave as expected. I don't know if this is a real problem.
