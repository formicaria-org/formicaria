# Licence

formicaria is dual-licensed under either of

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) ·
  <https://www.apache.org/licenses/LICENSE-2.0>)
- **MIT license** ([LICENSE-MIT](LICENSE-MIT) ·
  <https://opensource.org/licenses/MIT>)

at your option — the Rust ecosystem convention. Take whichever suits you: MIT is short and
universal; Apache-2.0 adds an explicit patent grant. You may use, modify, distribute, sell
or close-source this, with no obligation back to us beyond keeping the notice.

## Contribution

Unless you state otherwise, any contribution you intentionally submit for inclusion in the
work, as defined in the Apache-2.0 licence, shall be dual licensed as above, with no
additional terms or conditions.

## Third-party code

The shipped binaries statically link ~65 crates, overwhelmingly MIT and Apache-2.0. Both
licences require their notices to travel with a binary, so each release archive contains a
`THIRD-PARTY.md` generated from the actual dependency tree at build time. Nothing copyleft
is linked in — `deny.toml` gates that on every CI run.
