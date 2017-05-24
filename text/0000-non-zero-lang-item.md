- Start Date: 2014-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `NonZero` lang item to be used to hint to the compiler that the wrapped pointer or integral value is never 0/NULL.

# Motivation

We currently take advantage of the fact that we know certain kinds of types either are never NULL themselves (`Box`, `&`, function pointers) or contain some pointer which we know to never be NULL (the pointer in a slice, trait object, closure). Specifically, for certain kinds of enums the contain these types, we don't add in an extra field for the discriminant but rather non-nullness of the type to encode which of two possible variants the enum may be.

This is what enables `size_of::<Box<T>>() == size_of::<Option<Box<T>>>()`.

Unfortunately we can't take advantage of this in library code leading to extra overhead for `Option<Vec<T>>`, `Rc<T>` and the like.

# Detailed design

As part of this we'd introduce a new lang item:
```
#[lang = "non_zero"]
struct NonZero<T>(T);
```

Then, `trans::adt` can treat `NonZero` fields that wrap a pointer or integral type in the same manner as it currently does for `Box<T>` and the like. Thus, we'd represent such enums as `RawNullablePointer/StructWrappedNullablePointer`.

In using it, creating an instance of `NonZero` would be an unsafe operation (private field, mark `::new` as unsafe). We could also implement `Deref` for convenience (i.e. for `foo: NonZero<*mut i8>`, `*foo` would return `*mut i8` since `*mut i8` is `Copy`).

# Drawbacks



# Alternatives

We could possibly do this with an attribute instead but it's a bit more annoying to mark it as unsafe beyond just the name or hardcoding that specific attribute in rustc.

# Unresolved questions
