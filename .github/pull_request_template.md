<!-- The diff says what changed. This says why, and what you checked. -->

## What and why

## How it was checked

- [ ] `pixi run ci` is green (it is the whole gate — nothing runs on a push here)
- [ ] Any new test was **seen to fail without the fix**, or this says why it cannot be

<!-- If you touched commit_all or git_native.rs: `pixi run test-native-git` too.
     `cargo test --workspace` cfgs out the libgit2 backend, which is the one the phone uses. -->

## What this deliberately does not do

<!-- A gap you name is worth more than one a reviewer finds. -->

## The four questions

<!-- Only if this ADDS something: a dependency, a file in the release archive, a relaxed guard, a
     changed default. See CONTRIBUTING.md. Delete this section otherwise. -->

- Subject (`#seams` `#git` `#sync` `#track-m` `#ui` `#vault` `#data` `#toolchain` `#agent`):
- Verdict — permitted / extends an exception / contradicts a standing decision:
- If it widens what ships, or contradicts: the `decisions.md` entry is in this PR.
