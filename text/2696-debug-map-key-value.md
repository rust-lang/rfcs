- Feature Name: `debug_map_key_value`
- Start Date: 2019-05-01
- RFC PR: [rust-lang/rfcs#2696](https://github.com/rust-lang/rfcs/pull/2696)
- Rust Issue: [rust-lang/rust#62482](https://github.com/rust-lang/rust/issues/62482)

## Summary
[summary]: #summary

Add two new methods to `std::fmt::DebugMap` for writing the key and value part of a map entry separately:

```rust
impl<'a, 'b: 'a> DebugMap<'a, 'b> {
    pub fn key(&mut self, key: &dyn Debug) -> &mut Self;
    pub fn value(&mut self, value: &dyn Debug) -> &mut Self;
}
```

## Motivation
[motivation]: #motivation

The format builders available to `std::fmt::Debug` implementations through the `std::fmt::Formatter` help keep the textual debug representation of Rust structures consistent. They're also convenient to use and make sure the various formatting flags are retained when formatting entries. The standard formatting API in `std::fmt` is similar to `serde::ser`:

- `Debug` -> `Serialize`
- `Formatter` -> `Serializer`
- `DebugMap` -> `SerializeMap`
- `DebugList` -> `SerializeSeq`
- `DebugTuple` -> `SerializeTuple` / `SerializeTupleStruct` / `SerilizeTupleVariant`
- `DebugStruct` -> `SerializeStruct` / `SerializeStructVariant`

There's one notable inconsistency though: an implementation of `SerializeMap` must support serializing its keys and values independently. This isn't supported by `DebugMap` because its `entry` method takes both a key and a value together. That means it's not possible to write a `Serializer` that defers entirely to the format builders.

Adding separate `key` and `value` methods to `DebugMap` will align it more closely with `SerializeMap`, and make it possible to build a `Serializer` based on the standard format builders.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In `DebugMap`, an entry is the pair of a key and a value. That means the following `Debug` implementation:

```rust
use std::fmt;

struct Map;

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        map.entry(&"key", &"value");

        map.finish()
    }
}
```

is equivalent to:

```rust
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        // Equivalent to map.entry
        map.key(&"key").value(&"value");

        map.finish()
    }
}
```

Every call to `key` must be directly followed by a corresponding call to `value` to complete the entry:

```rust
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        map.key(&1);

        // err: attempt to start a new entry without finishing the current one
        map.key(&2);

        map.finish()
    }
}
```

`key` must be called before `value`:

```rust
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        // err: attempt to write a value without first writing its key
        map.value(&"value");
        map.key(&"key");

        map.finish()
    }
}
```

Each entry must be finished before the map can be finished:

```rust
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        map.key(&1);

        // err: attempt to finish a map that has an incomplete key
        map.finish()
    }
}
```

Any incorrect calls to `key` and `value` will panic.

### When to use `key` and `value`

Why would you want to use `key` and `value` directly if they're less convenient than `entry`? The reason is when the driver of the `DebugMap` is a framework like `serde` rather than a data structure directly:

```rust
struct DebugMap<'a, 'b: 'a>(fmt::DebugMap<'a, 'b>);

impl<'a, 'b: 'a> SerializeMap for DebugMap<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.key(&key.to_debug());
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.0.value(&value.to_debug());
        Ok(())
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        self.0.entry(&key.to_debug(), &value.to_debug());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.finish().map_err(Into::into)
    }
}
```

Consumers should prefer calling `entry` over `key` and `value`.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `key` and `value` methods can be implemented on `DebugMap` by tracking the state of the current entry in a `bool`, and splitting the existing `entry` method into two:

```rust
pub struct DebugMap<'a, 'b: 'a> {
    has_key: bool,
    ..
}

pub fn debug_map_new<'a, 'b>(fmt: &'a mut fmt::Formatter<'b>) -> DebugMap<'a, 'b> {
    DebugMap {
        has_key: false,
        ..
    }
}

impl<'a, 'b: 'a> DebugMap<'a, 'b> {
    pub fn entry(&mut self, key: &dyn fmt::Debug, value: &dyn fmt::Debug) -> &mut DebugMap<'a, 'b> {
        self.key(key).value(value)
    }

    pub fn key(&mut self, key: &dyn fmt::Debug) -> &mut DebugMap<'a, 'b> {
        // Make sure there isn't a partial entry
        assert!(!self.has_key, "attempted to begin a new map entry without completing the previous one");

        self.result = self.result.and_then(|_| {
            // write the key

            // Mark that we're in an entry
            self.has_key = true;
            Ok(())
        });

        self
    }

    pub fn value(&mut self, value: &dyn fmt::Debug) -> &mut DebugMap<'a, 'b> {
        // Make sure there is a partial entry to finish
        assert!(self.has_key, "attempted to format a map value before its key");

        self.result = self.result.and_then(|_| {
            // write the value

            // Mark that we're not in an entry
            self.has_key = false;
            Ok(())
        });

        self.has_fields = true;
        self
    }

    pub fn finish(&mut self) -> fmt::Result {
        // Make sure there isn't a partial entry
        assert!(!self.has_key, "attempted to finish a map with a partial entry");

        self.result.and_then(|_| self.fmt.write_str("}"))
    }
}
```

## Drawbacks
[drawbacks]: #drawbacks

The proposed `key` and `value` methods are't immediately useful for `Debug` implementors that are able to call `entry` instead. This creates a decision point where there wasn't one before. The proposed implementation is also going to be less efficient than the one that exists now because it introduces a few conditionals.

On balance, the additional `key` and `value` methods are a small and unsurprising addition that enables a set of use-cases that weren't possible before, and aligns more closely with `serde`.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The universal alternative of simply _not doing this_ leaves consumers that do need to format map keys independently of values with a few options:

- Write an alternative implementation of the format builders. The output from this alternative implementation would need to be kept reasonably in-sync with the one in the standard library. It doesn't change very frequently, but does from time to time. It would also have to take the same care as the standard library implementation to retain formatting flags when working with entries.
- Buffer keys and format them together with values when the whole entry is available. Unless the key is guaranteed to live until the value is supplied (meaning it probably needs to be `'static`) then the key will need to be formatted into a string first. This means allocating (though the cost could be amortized over the whole map) and potentially losing formatting flags when buffering.

Another alternative is to avoid panicking if the sequence of entries doesn't follow the expected pattern of `key` then `value`. Instead, `DebugMap` could make a best-effort attempt to represent keys without values and values without keys. However, this approach has the drawback of masking incorrect `Debug` implementations, may produce a surprising output and doesn't reduce the complexity of the implementation (we'd still need to tell whether a key should be followed by a `: ` separator or a `, `).

## Prior art
[prior-art]: #prior-art

The `serde::ser::SerializeMap` API (and `libserialize::Encoder` for what it's worth) requires map keys and values can be serialized independently. `SerializeMap` provides a `serialize_entry` method, which is similar to the existing `DebugMap::entry`, but is only supposed to be used as an optimization.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

## Future possibilities
[future-possibilities]: #future-possibilities

The internal implementation could optimize the `entry` method to avoid a few redundant checks.
