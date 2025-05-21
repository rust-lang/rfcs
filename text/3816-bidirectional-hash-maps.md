- Feature Name: (`bidirectional_hash_maps`)
- Start Date: (2025-05-21)
- RFC PR: [rust-lang/rfcs#3816](https://github.com/rust-lang/rfcs/pull/3816)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
This RFC suggests a new data structure for Rust, to be implemented later: A Bi-Directional HashMap, or BijectionMap<K, V, S> where both the key and value are hashable and can be looked up. 

# Motivation
Many times, people would wish for a simple reverse-lookup function to convert their enums to different values, perhaps a String to their enum values and vice versa. In the current implementation, you would require two hashmaps:

```rust
use std::collections::HashMap;

pub enum MyEnum {
    VariantA,
    VariantB,
    VariantC,
    VariantD,
    // and so on...
}

const STRING_TO_MYENUM_MAP: HashMap<&str, MyEnum> = HashMap::from([("a", MyEnum::VariantA), ("b", MyEnum::VariantB), ("c", MyEnum::VariantC), ("d", MyEnum::VariantD)] /* and so on... */);
const MYENUM_TO_STRING_MAP: HashMap<MyEnum, &str> = HashMap::from([(MyEnum::VariantA, "a"), (MyEnum::VariantB, "b"), (MyEnum::VariantC, "c"), (MyEnum::VariantD, "d")] /* and so on...*/);
This is too cumbersome for many people to use. True, they could implement their own method, but could there be a better solution using just one map? They could define it

use std::collections::BijectionMap;

pub enum MyEnum {
    VariantA,
    VariantB,
    VariantC,
    VariantD,
}

const STRING_CONV_MYENUM_MAP: BijectionMap<&str, MyEnum> = BijectionMap::from([("a", MyEnum::VariantA), ("b", MyEnum::VariantB), ("c", MyEnum::VariantC), ("d", MyEnum::VariantD)] /* and so on... */);
```
and use it here:

```rust
let from_string = STRING_CONV_MYENUM_MAP.get_value("a");
let to_string = STRING_CONV_MYENUM_MAP.get_key(MyEnum::VariantA);
```

# Guide-Level Explanation
A BijectionMap is a Rust collection that maps keys and values together. Unlike a HashMap, you can look up a BijectionMap both ways, by key or by value.

Say you're developing a program and you need to check usernames and match them up with user emails and vice versa. You can, obviously, use two hashmaps and develop a reverser function. But why do two if you can use just one map?

```rust
let USER_BIDI_MAP = BijectionMap::new::<&str, &str<()
for user in users {
    USER_BIDI_MAP.insert(user.username, user.email);
}
let random_user_who_logged_in = "iamaverysmartperson";
let their_email USER_BIDI_MAP.get(random_user_who_logged_in);
It can also be used to convert enums to other values:

pub enum IpResponse<Value, Error> {
   Success(Value),
   Info(Value),
   Err(Error)
}

// let statuscode_equals_ipresponse = blah blah blah
// This can help us get a status code, convert it to an enum, use it in functions, and send it to a user downstream for example
```
# Reference-Level Explanation
BijectionMap<K, V, S> will have both Keys and Values implement Eq and Hash, but the API is quite similar to HashMap:

```rust
let ikea_bijection_map = BijectionMap::new::<&str, u32>();
ikea_bijection_map.insert("SVALLJARD", 8321789228129);
ikea_bijection_map.insert("STENKOL", 182317198732918);
ikea_bijection_map.insert("VALLHEIOR", 92131831298718);
ikea_bijection_map.insert("HAREDROR", 92311273981273);

// this user wants WELTAEK. Now which one is that?
user.get_key("samanheik288").cart.push_back(ikea_bijection_map.get_key("WELTAEK"));

// one of the delivery agents gave us this product number and this username. What does it match to?
user.get_key("heldros8492").cart.push_back(ikea_bijection_map.get_value(923179812371));

// ATTENTION! SAMANKEIV is no longer being sold!
ikea_bijection_map.remove_key("SAMANKEIV");

// We ran out of 9328193103930 and IKEA will stop selling it here. What do we do?
ikea_bijection_map.remove_value(9328193103930);

// ERWAK is a great product... but is the name being used?
ikea_bijection_map.entry_key("ERWAK").or_insert(21389181329871);
```

# Drawbacks
None?

# Rationale and Alternatives
An alternative is to implement two or three hashmaps for looking up each value. The BijectionMap reduces memory size and typing, and is easier to understand in terms of being explicit.

# Prior art
[bidirectional_map](https://docs.rs/bidirectional_map/latest/bidirectional_map)

# Unresolved questions
None? This is quite similar to a HashMap.

# Future possibilities
If variadic generics are ever stabilised, a new MultidirectionalMap may be created with variadic generics for the types, to be able to convert between a theoretically infinite number of types. However, the functions for this will be inevitably complicated.

