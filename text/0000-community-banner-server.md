- Feature Name: community-banner-server
- Start Date: 2017-08-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Provide a server to promote recurring information into the community in the form of banners. This allows us to publish info
that we want to spread constantly more consistently.

# Proposal
[proposal]: #proposal

## The problem: recurring messaging

The usual way we promote new things is: "Hey, we've got the new thing here, please participate" - and then never again. This gives us a lot of spread at the time of publication, but then, we rely on person-to-person spread. Indeed, we are at a state where even seasoned people in the community suddenly hear of something that was announced quite a while ago. One-shot messaging is also often missed, as people might not be awake if it goes around.

Newcomers never hear about these campaigns, as - well, they weren't there at the time of the announcement. They only ever hear about things if:

* They raise the subject to someone that knows that there is something going on
* If someone directly talks to them

Examples of these things are:

* The Forge (https://forge.rust-lang.org)
* Servo Starters (https://starters.servo.org)
* CFPs and Ticket sales for conferences
* Promotion of upcoming or ongoing events like RustBridge

The solution to this is recurring messaging: regular spread the message, by re-linking to the announcements or pages of the project. For example, a Twitter account might tweet about a new announcement three times, for different time-zones. This cannot go on indefinitely, as social media accounts are built for an ongoing stream.

## Constant recurring messaging: banners

We're already, in the hiding, employing banner-like structures for this: for example, the main rust-lang page has a section that constantly presents a different language feature, forever and ever. Banners are an appropriate place for recurring messaging: they are always at the same place, they can hold different content and they can be placed at different locations.

Banners have a bad rep because they are usually employed in a very intrusive fashion and don't give much context, but that doesn't make them a generally bad thing. Relevant banners are helpful and welcome, but great care must be taken to keep them relevant. Great care must be taken to optimize the "wow, today I learned"-factor high and keep the annoyance of people that already know about the thing down.

A primary goal here must be to keep the managing work low and easy.

One way to do this is having a central authority that handles publishing and un-publishing and then spreads to publishers.

Specialised ad publishers are not a unusual and regularly have a better rep then large ad networks, because they can easily control relevance and annoyance.

## Example Implementation

https://github.com/rust-community/rust-campaigns-server/ is a server serving recurring messages relevant to the Rust community. It doesn't track anything and currently does no analysis about the location of the user. For ethical reasons, I'd like to be very conservative on this side, especially as impact analysis isn't the main reason behind this.

It's currently functional with a console only admin backend. There is an exemplary javascript embed code available, which can be used to put an unstyled banner on any website. It also has an API for custom creation of banners. Its goal is to be conscious of users bandwidth, we don't want to apply a lot of styling and find ways to aggressively use browser caches.

These embed codes can be used on project pages or the pages of interested community members.

There's prior art for this, namely the [Perl Community Ad Server](http://pcas.szabgab.com/).

# Drawbacks
[drawbacks]: #drawbacks

Such a service must be maintained and kept running after committing to it, as people choosing to embed these banners
rely on them not breaking their website.

Review must be done for each an every ad, which requires some
people committed to this.

# Rationale and Alternatives
[alternatives]: #alternatives

Keep things like they are.

# Unresolved questions
[unresolved]: #unresolved-questions

What would be an acceptable policy to measure impact of this?
