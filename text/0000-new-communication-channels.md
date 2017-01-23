- Feature Name: new_communication_channels
- Start Date: 2017-01-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Traditionally, most of Rust's main community interaction has been via the Rust IRC channels. These
channels have spanned everything from beginner topics to the core team meetings. Unfortunately,
there are current contributors to Rust and potential users to Rust who are hindered by this heavy
focus on IRC. This RFC proposes creating additional official Rust communication channels.

# Motivation
[motivation]: #motivation

The motivation is fairly simple: IRC is struggles in terms of usability, and to some, this is a
barrier for entry.  More recent chat platforms like gitter and Slack have filled in the usability
gap, and are now preferred over IRC.

At the same time, core developers spend a fair amount of time discussing issues on IRC, so it's an
essential place to participate if you want to be deeply involved in the project. By requiring people
to use IRC, we're creating an boundary between the Rust community and potential contributors.

If our goal is to create a warm, welcoming community, then part of that work means working with
technologies that are more familiar and comfortable to this potential community.

# Detailed design
[design]: #detailed-design

This RFC proposes that we create two new Rust communication channels: an official Slack channel and
we make the [unofficial gitter channel](https://gitter.im/rust-lang/rust) official.

Slack has been a request from some new users because they're more used to it, as it's grown
in popularity with many software companies. This allows them to more seamlessly transition their
habits to contributing to Rust.

Gitter has grown to over 1200 registered people, the same size as the official #rust channel,
all without any official Rust promotion.  This seems to point to a potential for Rust to be able to
reach out to users who prefer alternatives to IRC.

These two new channels would cater to helping new Rust users as well as offer a point contact for
advanced Rust contributors to interact with each other. This will happen through the use of rooms.
The default room will be the one new users will see first. This is where we can help folks with
their general Rust questions and is a catch-all to help direct people to the appropriate room or
other source of information.

The new channels would also have additional rooms. At first, this will likely be only one additional
room focused on contributing to Rust (like #rust-internals), though we may also want to create the
equivalent of #rust-beginners as well.

The goal is to create helpful landing pads for people comfortable with gitter and Slack, and a place
for people to grow into Rust contributors and still use their communication channel of choice.


# Drawbacks
[drawbacks]: #drawbacks

Prior to this RFC, there have been
[on-going discussions](https://users.rust-lang.org/t/a-possible-rust-slack-channel/7433) about
standing up new communication channels. To summarize the drawbacks:

* This creates a fork in the community where part of the community will use one communication
channel rather than another, which risks splintering us.

This is a risk, though it assumes that the community won't expand its scope to accomodate. Some of
us will likely just add a few more browser tabs for the new channels so that as a whole switching
between then becomes largely a non-issue over time.

* This may create an "us vs them" where all the "good stuff" happens on one of the other
communication channels.

Some of this is natural and avoidable. Already there is plenty of good Rust work going on that is
in other languages, other locales, and even just other times of day than what any one person can
experience.

This is in part why I suggest we make these official channels and encourage the community to use
them. The best way for them to not feel second tier is to keep the quality high and keep the channel
feeling like the same welcoming Rust experience they would get on IRC.

* There is some concern specifically about Slack and its role in open source, as it is a closed
platform.

Since the RFC recommends continuing to use IRC, advocates of IRC can continue to use it. Rather,
this RFC attempts to expand to welcome Slack users in addition to IRC users. Rust is supported by a
number of closed-source solutions (eg GitHub, twitter, cloud infra, etc), and we can optionally
seek out alternatives if the closed-source solution begins to hinder the community and Rust's
growth.

* These new channels increase the moderation burden and may require the moderation team to grow to
accomodate.

# Alternatives
[alternatives]: #alternatives

One alternative is to use unofficial, rather than official, channels. The drawback of this approach
is that if we don't promote them how will the users they're intended to reach out to find them and
how will the sense of equality we want to reach be fostered?

There has also been discussion about possibly not only creating the new channels but in a sense
unifying them using bots that can bridge between them. Unfortuantely, the current state of these
bots seems to not be at the level of quality to give a good user experience.

Lastly, this RFC proposes creating new official communication channels, but it does not address
where the subteams meet.  A variant of this proposal could suggest that, for example, Slack be where
the subteams meet.  The difficulty here is that there is no consensus around the best communication
channel for subteam meetings.  As there are likely a number of possibilities here (eg alternating
in the same way you might alternate meeting times to accomodate different timezones), I propose we
leave this topic to be explored in follow-up proposals.

# Unresolved questions
[unresolved]: #unresolved-questions

