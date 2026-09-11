A release manifest signed by **openssl** with a **throwaway** Ed25519 key, whose private half was
deleted the moment this was written. It exists to pin one fact: that the raw 64-byte signature
`ci/release-sign.sh` produces with `openssl pkeyutl -rawin` is one `ring` accepts. The public key here
is not, and must never be, listed in `release-keys.txt`.
