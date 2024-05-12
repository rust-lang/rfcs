- Feature Name: rust-lang_github_org_access_policy 
- Start Date: 2020-03-02 

# Summary
[summary]: #summary

This RFC proposes a policy for managing permissions to the [Rust-Lang GitHub Organization](https://www.github.com/rust-lang) and repositories within this organization.

This RFC was written in consultation with the Governance Working Group and the Infrastructure team. Most discussion took place on [this issue](https://github.com/rust-lang/wg-governance/issues/4) and [this pull request](https://github.com/rust-lang/wg-governance/pull/42).

# Motivation
[motivation]: #motivation

Access control for the [Rust-Lang GitHub Organization](https://www.github.com/rust-lang) and repositories within that organization is currently managed either through the [rust-lang team database][db], or ad-hoc via the GitHub UI by the org owners. We need a policy that defines how these accesses are granted and managed. This will allow us to have greater security in permissions to our GitHub org, and provide transparency and clarity on how access is managed.

[db]: https://github.com/rust-lang/team/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Rust-Lang GitHub Permissions Policy

This policy applies to both the [Rust-Lang GitHub Organization](https://github.com/rust-lang/) and all repositories within that organization.

### Rust-Lang Organization

Access to the Rust-Lang GitHub organization is managed with the [rust-lang team database][db]. The team database is managed by the [team-repo-admins], whose policies are specified in the [Team Maintenance] documentation.

Selected members of the [Infrastructure Team] can also be organization owners if their work requires it.

All GitHub accounts used to interact with the Rust-Lang GitHub organization (owner or non-owner) must have 2FA enabled.

[team-repo-admins]: https://github.com/rust-lang/team/blob/master/teams/team-repo-admins.toml
[Team Maintenance]: https://forge.rust-lang.org/infra/team-maintenance.html
[Infrastructure Team]: https://github.com/rust-lang/team/blob/master/teams/infra.toml

### Rust-Lang Repositories

Access to and permissions for repositories within the Rust-Lang organization must be administered through the [rust-lang team database][db]. Permissions should not be given to individuals, only to teams or groups.

GitHub provides several permission levels for access to a repository. Please refer to [GitHub's documentation](https://help.github.com/en/github/setting-up-and-managing-organizations-and-teams/repository-permission-levels-for-an-organization) for details on permission levels and what each level can do.

Repositories in the Rust-Lang organization should follow these permission guidelines:

* **Admin** --- No users or teams except for org owners should have this permission level.
* **Maintain** --- Teams may have this permission level at their discretion for repositories the team is responsible for.
  Repositories using the [bors] bot may want to consider using the *write* permission level instead in order to deactivate the "Merge" button on PRs to enforce that merges go through bors.
* **Write** --- Teams that are responsible for a repository should have at least this permission level.
* **Triage** --- This role is available if teams want to give these permissions to other teams, such as for triage support. Unfortunately this role does not allow contributors to edit issue descriptions or titles, so its utility for that purpose is limited.
* **Read** --- This role is unnecessary, and should not be used (it is generally only relevant to private repositories, and we do not have a use case for it).

Teams who are responsible for a repository may give access to other teams at their discretion.

Teams or groups may ask for repositories to be created to fulfill their needs by opening a PR to the [Team Repository][db]. It is up to the team-repo-admins to approve creating the repositories. Existing repositories that need to be transferred from outside the rust-lang organization should consult with the Infrastructure Team to fulfill that request.

By default, repositories should be public and allow read access to all. When needed, some repositories can have limited read access (i.e. repositories related to security). 

Some teams - such as the moderation team - need broad access to public Rust-Lang repositories. The first way to manage this is through creating a GitHub team managed through the [Team Repository][db] and granting that team appropriate permissions to the appropriate repos. Another way is to create tooling that will allow a member of the moderation team to selectively and temporarily gain the access that they need when it is needed (such as deleting a comment or issue). For now, we are proceeding with managing access to repos for moderation through a GitHub team, however, should it be needed, we can develop tooling to apply more fine grained and time limited access.

Bot accounts controlled by the Infrastructure Team (such as the [triagebot]) can be granted any level of access required for them to work at the discretion of the Infrastructure Team.

[bors]: https://github.com/rust-lang/homu
[triagebot]: https://forge.rust-lang.org/triagebot/index.html

## Implementation

It is the responsibility of the Leadership Council, the Infrastructure Team, and the team-repo-admins to finish the migration to implement this policy. New teams may need to be created, which is outside the scope of this RFC to define.

# Drawbacks
[drawbacks]: #drawbacks

There can be exceptional cases where a team wants to give repository access to an individual to assist with their work. Requiring them to join or create a team in order to perform that work can be a significant hassle. Teams who find they need this frequently should consider creating a "contributors" subteam for that purpose, or to investigate other tooling to assist with what they need.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should these rules applied to Rust-Lang affiliated repositories and organizations that are outside of the [Rust-Lang GitHub Org](https://www.github.com/rust-lang), such as [rust-embedded](https://github.com/rust-embedded)?

# Future possibilities

- [Custom GitHub Roles](https://docs.github.com/en/enterprise-cloud@latest/organizations/managing-user-access-to-your-organizations-repositories/managing-repository-roles/about-custom-repository-roles) could be created for use cases where the existing roles do not suffice.
- Extend tooling, such as [triagebot], to provide extended permissions that are not normally available (for example, it currently offers [labeling](https://forge.rust-lang.org/triagebot/labeling.html)).
