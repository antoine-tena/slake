# ion-render

ion-repo's template engine, in Rust. It renders a [Copier](https://copier.readthedocs.io)-format
template and updates the projects made from it with a three-way merge, for
[ion-repo](https://github.com/antoine-tena/ion-repo).

> **Work in progress, not usable yet.** ion-repo keeps running Copier until ion-render renders every ion-repo
> stack byte for byte the same, and replays every ion-repo release since v0.3.0.

## Why

`ion repo update` spends about 80% of its time in Copier: 11 to 12 seconds on a small repo, where the
template's mirror is refreshed five times and most of the CPU goes to compiling and rendering
Jinja templates. ion-repo uses a small, fixed part of Copier, and works around it in five places:
renames across releases, a spurious conflict at each promotion, conflict markers searched for
after the fact, its own laying for adopted repos, extra renders for frozen files. An engine built
for that part can be much faster, and give ion-repo the update it needs:

- one fetch per command, from a cached bare mirror of the template;
- each revision rendered once per command, with
  [minijinja](https://github.com/mitsuhiko/minijinja);
- stable revisions: the answers file records the revision ion-repo asked for, and an update renders
  the old one under that name, so a promoted tag no longer brings a spurious conflict;
- Copier's inline three-way update, plus a JSON report of what was added, updated, deleted and
  conflicted, and of what changed in excluded files;
- excluded paths, declared renames and an adopt mode as first-class inputs;
- a single binary in a wheel, like ruff, called by ion-repo with JSON in and out.

Targets, measured with hyperfine: `ion repo update` on a small repo from 11–12 s to 5 s or less,
`ion repo new` from 5 s to 3.5 s or less.

## Usage

`ion render <command>` reads one JSON request on its standard input and writes one JSON response on
its standard output; [docs/contract.md](docs/contract.md) describes them. `ion render render` is the
one command so far: it renders a revision into a new directory, the same files as
`copier copy --defaults`, byte for byte.

```sh
echo '{"contract": 1,
       "template": {"url": "https://github.com/antoine-tena/ion-repo.git", "ref": "v0.4.9"},
       "answers": {"name": "demo"}, "destination": "demo"}' | ion-render render
```

Build it with `cargo build --release` (the toolchain is pinned in `rust-toolchain.toml`).

## License

[MIT](LICENSE)
