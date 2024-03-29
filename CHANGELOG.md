## next

* workspace dependency support
* highlight empty packages
* search for packages

## 0.6.0

* use `cargo metadata` instead of custom parsing. This helps a lot for edge cases e.g. git
* handle git dependencies
* fix bug where changes where not saved when filtering dependencies
* handle build-dependencies and dev-dependencies
* display which dependencies a feature will enable
* run `cargo test` for `cargo features prune`

## 0.5.3

* display dependency-parsing-error next to dependency instead of crashing
* ignore `no dependencies were found` when working in a workspace

## 0.5.2

* don't crash when using workspace dependencies
* display workspace dependencies as package
* handle one cargo.toml being a workspace and a package

## 0.5.1

* allow * to be in workspace path
* fix local dependencies resolution

## 0.5.0

* handle workspaces
* always sort features
* add basic terminal autocompletion

## 0.4.0

* `cargo features prune` see [README.md](README.md#prune)
* move from `crossterm` to `console`

## 0.3.3

* only fetch crates when needed

## 0.3.2

* update sparse-cache

## 0.3.1

* sparse index

## 0.3.0

* better search algorithm
* change navigation keys

## 0.2.0

* `search` see [README.md](README.md#search-mode)

## 0.1.2

* support optional features

## 0.1.1

* only save changed features
* better gray color

## 0.1.0

* initial release