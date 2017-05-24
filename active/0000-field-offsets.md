- Start Date: 2014-07-21
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This adds the ability to refer to fields of types and use them later on with objects of that type.
The `FieldOffset<Obj, Field>` type is added which refers to fields of type `Field` in the object `Obj`.
The `offsetof Type.field` syntax is defined to construct instances of `FieldOffset<Type, the type of field>`.

# Motivation

Linked lists with internal storage. In this example we create an object and let
it be a member of both linked lists using while using no heap allocations.
Care must be taken here to ensure the lifetime of `obj` outlives all lists it's an
member of.

```rust
struct ListData {
    next: Cell<*()>
}

struct List<Object> {
    first: *const Object
    field: FieldOffset<Object, ListData>
}

impl<Object> List<Object> {
    fn add(&self, obj: &Object) {
        field.get(obj).set(mem::transmute(self.first));
        self.first = obj;
    }
    
    fn new(field: FieldOffset<Object, ListData>) {
        List::<Object> { field: field }
    }
}

struct Object {
    list_a: ListData;
    list_b: ListData;
}

fn main() {
    let obj = Object { list_a: ListData::new(), list_b: ListData::new() }

    let mut a = List::new(offsetof Object.list_a);
    let mut b = List::new(offsetof Object.list_b);
    
    a.add(&obj);
    b.add(&obj);
}
```

# Detailed design

Suggested interface:
```rust
pub struct FieldOffset<Obj, Field>;

impl FieldOffset<Obj, Field> {
    pub fn get(&self, obj: &Obj) -> &Field;
    pub fn get_mut(&self, obj: &mut Obj) -> &mut Field;
    pub fn get_raw(&self, obj: *const Obj) -> *const Field;
    pub fn get_raw_mut(&self, obj: *mut Obj) -> *mut Field;
}
```

Example use:
```rust
struct Test {
    field: int
}

fn main() {
    let off: FieldOffset<Test, int> = offsetof Test.field;

    let mut t = Test { field: 1 }

    *off.get_mut(&t) = 2;

    println!("field is {}", t.field);
}
```

# Drawbacks

One more builtin type and `offsetof` won't be usable for something weird.

# Alternatives

An alternative is to implement this using unsafe code. This would work if `typeof` did:
```rust
#![feature(macro_rules)]

use std::kinds::marker::InvariantType;
use std::mem::transmute;

struct FieldOffset<Obj, Field> {
    offset: uint,
    a: InvariantType<Obj>,
    b: InvariantType<Field>
}

impl<Obj, Field> FieldOffset<Obj, Field> {
    pub fn get(&self, obj: &Obj) -> &Field {
        unsafe {
            transmute(self.get_raw(obj))
        }
    }
    pub fn get_mut(&self, obj: &mut Obj) -> &mut Field {
        unsafe {
            transmute(self.get_raw_mut(obj))
        }
    }
    pub fn get_raw(&self, obj: *const Obj) -> *const Field {
        unsafe {
            transmute(obj.to_uint() + self.offset)
        }
    }
    pub fn get_raw_mut(&self, obj: *mut Obj) -> *mut Field {
        self.get_raw(obj as *const Obj) as *mut Field
    }
}

macro_rules! offset_of(
    ($ty:ty, $field:ident) => (
        unsafe {
            let null: &$ty = transmute(0u);
            let offset = &null.$field;
            FieldOffset {
                offset: transmute(offset),
                a: std::kinds::marker::InvariantType::<$ty>,
                b: std::kinds::marker::InvariantType::<typeof(*offset)>
            }
        }
    );
)

```
Without `typeof` support, the field type could be explicitly mentioned in the macro.

However if values in type parameters is going to be allowed it is desirable to pass it as type parameters, but that can't work with unsafe code.
In the list example above you could specialize the list type for a specific field, like `List<offsetof Object.list_a>`.
This generates more efficient code since the list won't have to look up the field offset at runtime.
An alternative to that again is to do something like this using traits:
```rust
trait FieldOffset<Obj, Field> {
    pub fn get(obj: &Obj) -> &Field;
}

struct Object {
  ListData list_a;
  ListData list_b;
}

struct ObjectListA;

impl FieldOffset<ListData> for ObjectListA {
    pub fn get(obj: &Obj) -> &Field {
        &obj.list_a
    }
}

struct ObjectListB;

impl FieldOffset<ListData> for ObjectListB {
    pub fn get(obj: &Obj) -> &Field {
        &obj.list_b
    }
}

struct List<Obj, Field: FieldOffset<Obj, ListData>>;

static list: List<Object, ObjectListA>;
```

Another alternative is to allow `offsetof Test.field` to return an `uint` which could be passed as a type parameter.
However that requires the compiler to know the layout of fields at the type checking stage.

# Unresolved questions

None.