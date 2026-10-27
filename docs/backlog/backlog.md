# Backlog

Everything left to do on slake, in a single list ordered by priority. Started on
**2026-10-26**. How to write a ticket: [README.md](README.md); closed tickets are in
[done.md](done.md), what was turned down in [rejected.md](rejected.md). **Nothing else is open.**

North star: **kiln runs on slake: every stack rendered byte for byte as Copier does, kiln update in 5 s or less**.

**Next free: T-10.**

---

## 🔴 Critical

## 🟠 Important

### T-09 · The parity bench in kiln, for every change of the template
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-27
- **Effort**: S · **Owner**: — · **Due**: —
- **Origin**: T-01 (the bench lives in kiln), T-02

T-02 was proven by a bench run by hand: each case of kiln's `tests/test_template.py` (41
stacks, podman, adopt with `kept`, sizes, user defaults, four refusals) rendered by Copier and
by `slake render`, the trees compared byte for byte (modes and empty directories included),
the response's `answers` and `files` checked. It goes into kiln as
`tests/test_slake_parity.py`, with the YAML fuzzer (PyYAML against `examples/probe.rs`) and
the Jinja2 snippets, so that every change of the template is rendered by both engines.

### T-03 · One fetch per command, from a cached bare mirror
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: S · **Owner**: — · **Due**: —
- **Origin**: the founding plan, item 1

A bare mirror per template URL in the user's cache, any revision read from it, no network when
the revision is already there. Each revision rendered once per command, shared by the update,
the drift report of excluded files and the renames (item 2).

`slake render` already reads from the mirror (`src/source.rs`): fetched only when the ref is
missing or names a branch, never with `offline`. Left: one render per revision and per
command, shared by the steps of T-04.

### T-04 · Update like Copier's inline mode, with a JSON report
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: L · **Owner**: — · **Due**: —
- **Origin**: the founding plan, items 4 to 6

Copier's `conflict="inline"` (`git merge-file` markers, the three versions in the index), plus a
report: files added, updated, deleted, conflicted, and what changed in excluded files (frozen
by `kiln-waivers.toml`, kept by adopt). The old revision is rendered under the name the answers
file recorded: no more spurious conflict at a promotion. Every existing `.copier-answers.yml`
(`_commit`, `_src_path`, answers, `kept`) is read as it is.

- [ ] kiln's `tests/test_e2e.py` replayed (every release since v0.3.0) with both engines, on
  throwaway clones only, never `antoine-tena/kiln-rehearsal` itself: same result.

### T-05 · Adopt mode and the question schema
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: M · **Owner**: — · **Due**: —
- **Origin**: the founding plan, items 8 and "what to drop"

Adopt: render, lay only the missing files, report the rest. Questions: no prompt in slake, only
the schema, with `when`, `validator` and computed defaults evaluated, so that kiln asks
`kiln new --all-questions` itself.

### T-06 · One binary in a wheel, for Linux and macOS
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: M · **Owner**: — · **Due**: —
- **Origin**: the founding plan, item 9

maturin `bindings = "bin"`, like ruff, for x86_64 and arm64; published on PyPI as `slake` (free
on 26 October 2026) by trusted publishing, with build provenance; kiln calls it with JSON in and
out.

### T-07 · kiln runs on slake, Copier as the fallback until parity
- **Type**: task
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: M · **Owner**: — · **Due**: —
- **Origin**: the founding plan, constraints and proof

In kiln: its five uses of Copier go through slake, Copier stays behind a setting until T-02 and
T-04 prove parity, then goes. It starts after kiln's public launch (11 November 2026): slake moves on in
its own repo meanwhile (antoine, 26 October 2026). No breaking change: same rendered files, `copier.yml` keeps its
format.

- [ ] Measured with hyperfine on throwaway copies, before and after: `kiln update` on a small
  repo 11–12 s → 5 s or less; `kiln new` 5 s → 3.5 s or less.

## 🟡 Normal

### T-08 · Declared renames applied during the update
- **Type**: improvement
- **Status**: todo
- **Opened**: 2026-10-26
- **Effort**: M · **Owner**: — · **Due**: —
- **Origin**: the founding plan, item 7

Answer keys, files and folders renamed across releases, declared by the template and applied
during the update, so that kiln's `migrations.py` shrinks to what is not a rename.

## ⚪ Background — long-running work, debt and ideas
