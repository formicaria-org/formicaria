# agents/ — where a checkout keeps the assistant's runtime and models

The assistant, **Myrme**, lives in its own repository:
**https://github.com/formicaria-org/myrme**. formicaria links it by a pinned tag (`Cargo.toml` of
`fm-serve`, `crates/agent-serve` and the phone shell), and its rulings, plan and evals are kept
there.

This folder is what a checkout uses while developing:
- `models.toml`, the catalogue, written by `pixi run catalogue` from the `agent-serve` this checkout
  builds. Do not edit it here; change it in Myrme.
- `runtime/` and `models/`, fetched by the app, or by Myrme's `pixi run fetch-model` pointed here.

All three are gitignored. To work on the assistant itself, clone Myrme. Its `pixi run integration`
builds this app at the release it pins and runs it against Myrme.
