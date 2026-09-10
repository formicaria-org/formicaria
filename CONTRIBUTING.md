# Contributing

Thanks for looking. A few things are worth knowing before you spend time on a change.

## What this project is

**One person's research notebook, built alongside their research.** It is used daily, it is not a
product, and it has a strong opinion about what it will not become. That makes some contributions
very welcome and others a waste of your afternoon — the section on what is *not* wanted is there to
save you the afternoon, not to be discouraging.

There is no roadmap you have to fit into, but there is a queue and a decision record, and both are
public: [`docs/context/outstanding.md`](docs/context/outstanding.md) is what is known to be
missing, [`docs/context/known-issues.md`](docs/context/known-issues.md) is what is known to be
wrong, and [`docs/context/decisions.md`](docs/context/decisions.md) is why things are the way they
are. Those are working notes rather than documentation — blunt, dated, and occasionally
contradicting each other on purpose, since the log is append-only.

## Getting set up

[**Getting started**](docs/src/dev/getting-started.md) in the developer guide is the fifteen-minute
version: install pixi, clone, `pixi run setup`, `pixi run serve`, `pixi run ci`.

The one rule the toolchain enforces: **it is pixi-only**. `cargo`, `node`, `pnpm` and `mdbook` are
pinned by `pixi.toml` and are not expected on your `PATH`. If you have your own, they are not the
ones this project builds with, and a bug reproduced against them is a bug in a different program.

`pixi run setup` installs a pre-commit hook that rejects any staged file over 5 MB. This repository
is text. Media belongs in the vault's content-addressed blob store, which is what keeps a clone
small a decade from now.

## The gate

```sh
pixi run ci
```

**That is the whole gate, and it runs on your machine.** Four of the five GitHub workflows —
`ci`, `cross`, `docs` and `ios` — are `workflow_dispatch:` only, so nothing checks your branch but
you. If `pixi run ci` is green, the change is testable.

**The fifth is `release.yml`, and it fires unattended on a `v*` tag.** Pushing a tag publishes a
release. It is the one workflow you can start by accident. It used to say "and bills three jobs";
that stopped being true when the repo went public, since GitHub does not charge for standard
runners there — but the tag still publishes, which is the part worth being careful about.

One trap worth repeating: `pixi run ci | tail` reports **`tail`'s** exit code, which is always 0.
Read the run's own status.

**And if the change touches anything environmental, run the gate as a fresh machine would:**

```sh
sh ci/like-a-runner.sh      # same gate, without what your machine happens to have
```

It withholds the things that were caught hiding real failures for eight weeks — your global git
identity, a leftover merge driver in `target/`, your locale, your core count. Everything passed
locally that whole time; the gate on a clean runner failed eight times for eight different reasons,
none of them a regression. Anything that shells out to git, times something, or sorts something is
worth this thirty-second check.

[Testing & CI](docs/src/dev/testing.md) covers what is inside the gate, the four pixi environments,
and four opt-in suites that are not in it — of which **`pixi run test-native-git` is the one to
remember**: `cargo test --workspace` `cfg`s out the libgit2 backend, which is the backend the phone
uses. Run it whenever you touch `commit_all` or `git_native.rs`.

## How work is expected to be shown

- **A test that has been seen fail.** A test that has never been red proves nothing, and this
  project has shipped green ticks that meant nothing. If a change fixes a bug, the accompanying
  test should fail without the fix, and it is worth saying in the pull request that you checked.
  Some things genuinely cannot be tested from here — say that instead, rather than writing a test
  that passes either way.
- **Say what you did not do.** A partial change with its gap named is far more useful than a
  complete-looking one with a quiet hole.
- **Commit messages carry the why.** The diff already carries the what.

## Before you add anything — four questions

This is the part that matters most, and it is the part an outside contributor cannot guess.

The project has ruled on a great many things, and the rulings are easy to contradict by accident,
because **every addition is justified locally and nothing checks the sum**. So before writing code
that *adds* something — a dependency, a file in the release archive, a relaxed guard, a changed
default:

1. **What subject is this?** Grep the `#subject` index at the top of `docs/context/decisions.md`:
   `#seams` `#git` `#sync` `#track-m` `#ui` `#vault` `#data` `#toolchain` `#agent`.
2. **Read the entries it names — and read past them.** The index is a starting point, not a
   contents page. (Found the hard way: the decisive ruling on desktop libgit2 is filed under
   `#track-m`, while the index lists the question under `#git`.)
3. **Classify honestly: permitted / extends an exception / contradicts a standing decision.** If
   everything you ever check comes back "permitted", you are not applying this.
4. **Does it widen what ships?** If yes it earns a `decisions.md` entry whatever the verdict.

**If it contradicts, write the dated reversal first** — and put a `> SUPERSEDED` banner on the
entry it supersedes. Writing it first forces the argument while changing course is still cheap;
writing it afterwards is how a project stops being what it said it was, one reasonable step at a
time.

`decisions.md` is append-only. A decision is superseded, never edited away: the value of *"we tried
X, then Y"* is the whole chain.

## The invariants a change must not break

Several are enforced by `ci/checks.sh`, so you will be told. They are listed here because knowing
*why* is faster than reading a grep's failure message:

- **`fm-query` may never touch `std::fs`, a database crate, or path types.** The query engine
  physically cannot do I/O. This is the whole insurance policy: the durable core cannot rot into a
  filesystem tangle.
- **Renderers stay literal-free.** No `todo`/`doing`/`done` or scheduling literals under
  `ui/src/renderers/`. A renderer that knows what a status *means* is a renderer that only works
  for one person's vocabulary.
- **Files are the truth, and the atom is the file.** One Markdown note, one file. This is the one
  invariant that voids the whole design if it changes — it is why real-time collaboration is not on
  the table, and it is why Logseq's block atom cost them three years.
- **A note read → edited → written back is byte-for-byte identical.**
- **Heavy tools are shelled out to, never linked** — `git`, `restic`, `pdftotext`,
  `vipsthumbnail`. Their often-GPL licences never enter the binary, and they upgrade independently.
- **Nothing silently discards a user's text.** A conflict puts both versions where a human can see
  them; a refused write says why.

## What is not wanted

Not a comment on the idea — these are settled, with the reasoning in `MASTERPLAN.md` and
`decisions.md`:

- **A third-party plugin API. Ever.** An unsandboxed plugin model is the attack surface a
  single-user tool never has to open.
- **A WYSIWYG or block editor.** Markdown stays canonical; a tree-first editor cannot guarantee a
  lossless round-trip.
- **Real-time collaborative editing.** It needs per-block ids, and the atom here is the file.
  Asynchronous collaboration shipped instead, over git.
- **Structural query over the note *body*.** Frontmatter is structured-queryable; the body is
  full-text only. If you want to filter on something, it goes in frontmatter.
- **A cloud service, an account, or a network dependency.**
- **A dependency that solves half a problem.** The bar is "solves it whole" — this codebase
  hand-rolls three environment lookups rather than take a crate for them, deliberately.

## Documentation

The manual under `docs/src/` is for **users**; `docs/context/` is the maintainer's working memory
and is not documentation. If your change alters what a user sees, the manual changes with it in the
same pull request. `pixi run docs` builds it.

`docs/src/reference/commands.md` claims to document every command, and `ci/checks.sh` now enforces
that — so a new `dispatch` arm needs a row.

## Code of conduct

[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) applies to everything here.
