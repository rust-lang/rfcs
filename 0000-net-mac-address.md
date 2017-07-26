- Feature Name: (net_address_mac)
- Start Date: 2017-07-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a media access control address (MAC) address datatype, `std::net::MacAddr48` to `std::net`.

# Motivation
[motivation]: #motivation

Currently there is no standard way to communicate physical (or ethernet) addresses when doing network related development in rust, even though `std::net::IpAddrV4` and `std::net::IpAddrV6` exist. The MAC address is, however, a regularly occurring data-structure when doing any type of network related development.

There is also a proliferation of implementations which are not compatible with each other, forcing developers to manually implement the data type again and again, reducing the opportunity for code re-use and convenience. `nom`[1], `libpnet`[2] and `diesel`[3] being a couple of examples.

[1] https://github.com/moosingin3space/pktparse-rs/blob/master/src/ethernet.rs

[2] https://github.com/libpnet/libpnet/blob/master/src/util.rs

[3] http://docs.diesel.rs/diesel/pg/types/sql_types/struct.MacAddr.html


# Detailed design
[design]: #detailed-design

It is proposed that the existing `crate` `eui48` be used (http://crate.io/eui48) as a basis for this RFC, thus the code below is copied directly from that implementation.

The following struct would be added to `std::net`:


```

/// A 48-bit (6 byte) buffer containing the EUI address
/// See: https://en.wikipedia.org/wiki/MAC_address
pub const EUI48LEN: usize = 6;
pub type Eui48 = [u8; EUI48LEN];

/// A MAC address (EUI-48)
#[derive(Copy, Clone)]
pub struct MacAddr48 {
    /// The 48-bit number stored in 6 bytes
    eui: Eui48,
}

```

It is proposed that most of the functions and `impl` from the `eui48` crate be included in `std::net::MacAddr48`, although there are open questions as to the need to support the `eui48` and `eui64` datatypes as those are trademarked by the IEEE, and MAC addresses are most commonly encountered in the ecosystem as well as the functions depending on Serde.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Networking related code is not directly addressed in any of the official rust books, that said, an effort could be made to contact some of the larger crates to encourage them to adopt the `std::net` structs.

It might be a good idea to investigate adding some examples to the rust cookbook, at https://brson.github.io/rust-cookbook/net.html#ex-random-port-tcp, altough the authors there would need to approve the topics and at this point no items in the standard library expose or use MAC addresses (based on a brief search), the target audience for this extension would primarily be other crate authors.

# Drawbacks
[drawbacks]: #drawbacks

Extending the standard library is something that should be very carefully considered before undertaking any changes, it increases the maintenance load on the relevant teams.

# Alternatives
[alternatives]: #alternatives

Promote or "bless" a standard crate for MAC addresses and spread the word to the large crates (such as diesel, libpnet, etc) and attempt to convince them to use it.

# Unresolved questions
[unresolved]: #unresolved-questions

Should we stop at MAC addresses? There are other networking datatypes to be included?
Should we add the `Serde` dependent serialization and deserialization `fn`'s?
