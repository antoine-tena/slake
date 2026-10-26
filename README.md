# slake

kiln's template engine, in Rust. It renders a [Copier](https://copier.readthedocs.io)-format
template and updates the projects made from it with a three-way merge, for
[kiln](https://github.com/antoine-tena/kiln).

> **Work in progress, not usable yet.** kiln keeps running Copier until slake renders every kiln
> stack byte for byte the same, and replays every kiln release since v0.3.0.

## Why

`kiln update` spends about 80% of its time in Copier: 11 to 12 seconds on a small repo, where the
template's mirror is refreshed five times and most of the CPU goes to compiling and rendering
Jinja templates. kiln uses a small, fixed part of Copier, and works around it in five places:
renames across releases, a spurious conflict at each promotion, conflict markers searched for
after the fact, its own laying for adopted repos, extra renders for frozen files. An engine built
for that part can be much faster, and give kiln the update it needs:

- one fetch per command, from a cached bare mirror of the template;
- each revision rendered once per command, with
  [minijinja](https://github.com/mitsuhiko/minijinja);
- exact revisions: the answers file records the commit, never a `git describe` name;
- Copier's inline three-way update, plus a JSON report of what was added, updated, deleted and
  conflicted, and of what changed in excluded files;
- excluded paths, declared renames and an adopt mode as first-class inputs;
- a single binary in a wheel, like ruff, called by kiln with JSON in and out.

Targets, measured with hyperfine: `kiln update` on a small repo from 11–12 s to 5 s or less,
`kiln new` from 5 s to 3.5 s or less.

## The name

Slaking is soaking dry clay in water so that it can be worked again: what an update does to a
project kiln made.

## License

[MIT](LICENSE)
