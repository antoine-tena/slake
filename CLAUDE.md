# slake

kiln's template engine, in Rust: renders a Copier-format template and updates the projects made
from it with a three-way merge. Its one consumer is kiln (cloned next to slake, public
`antoine-tena/kiln`), which will call the binary with JSON in and out.

<!-- This file belongs to the project: kiln wrote it once and never changes it. kiln's rules live
in .claude/rules/kiln/ and are kept current by `kiln update`. Write here what Claude cannot
guess about this project, and keep it under 200 lines: every session reads it. -->

## Repo map

Where to look, so as not to explore: read it before opening files, and update it in the commit
that adds, moves or removes a directory, a domain or an entry point. One line per place, what
it holds; no copy of the code.

- `README.md`: what slake is, why, its targets (public showcase); `LICENSE`: MIT.
- `docs/contract.md`: the JSON contract kiln calls slake with (version 1).
- `src/main.rs`: the binary, JSON in and out; `src/commands/`: one module per command
  (`render`); `src/contract.rs`: requests checked, responses written; `src/source.rs`: where a
  revision is read (cached mirror, local repository, directory).
- `src/template.rs`: a revision's files; `src/config.rs`: its `copier.yml` (settings refused,
  questions); `src/git.rs`: git's command line; `src/error.rs`: failures by kind.
- `src/questions/`: Copier's questionnaire (types, choices, validators); `src/answers.rs`,
  `src/cast.rs`: answers by priority and the render context.
- `src/render.rs`: paths, contents and modes of a render.
- `src/jinja/`: Jinja on minijinja as Copier configures it (Jinja2 and Ansible filters, Python
  methods, `list.append`); `src/posix.rs`: paths normalized like Python's `PurePosixPath`.
- `src/yaml/`: PyYAML's loader; `src/yaml/dump/`: its emitter, for `to_nice_yaml`.
- `src/data.rs`, `src/pyfmt/`: Python values and how Python writes them (`repr`, `json.dumps`).
- `examples/probe.rs`: a probe the fuzzing scripts compare with Python, value by value.
- `docs/backlog/`: the work to do; `charter.toml`: the checked rules; `.claude/rules/kiln/`:
  kiln's rules for Claude.

## Notes

- Parity before anything else: the same files as Copier, byte for byte, for every stack of
  kiln's `tests/test_template.py`, then a replay of kiln's `tests/test_e2e.py` (every kiln
  release since v0.3.0). Only throwaway clones are updated, never `antoine-tena/kiln-rehearsal`.
- No breaking change for kiln's repos: every existing `.copier-answers.yml` (`_commit`,
  `_src_path`, answers, `kept`) is read as it is, and `copier.yml` keeps its format. Copier
  stays kiln's fallback, behind a setting, until parity is proven.
- kiln has no Rust stack yet (kiln's T-320, T-321): `kiln check` runs `charter` only, and
  Rust's own checks are this repo's to run. A gap of kiln met here is fixed in kiln.
