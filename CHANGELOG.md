## 0.10.1

* add `--no-tmp` to prune

## 0.10.0

* run prune in temp folder
* hide cursor while pruning

## 0.9.1

* Add http2 to known false positives for
  rocket/hyper/hyper-util - [RivenSkaye](https://github.com/ToBinio/cargo-features-manager/pull/41)

## 0.9.0

* move prune progress display to bottom
* add `cargo features prune --clean <CLEAN>`

## 0.8.4

* fix handling of renamed dependencies in
  workspaces - [the-wondersmith](https://github.com/ToBinio/cargo-features-manager/pull/39)

## 0.8.3

* `cargo features prune` now runs all test
* add `--skip-tests` to prune

## 0.8.2

* fix `features-manager.keep` only being applied to normal dependencies

## 0.8.1

* fix search for features not working

## 0.8.0

* #### BREAKING - move Features.toml into Cargo.toml see [README.md](README.md#prune)

* use `color-eyre` instead of `anyhow`
* handle unused workspace dependencies
* allow `default` to be a sub_feature
* sort dependencies and packages alphabetically if no filter is set
* when running `cargo features prune` correctly handle sub features
* improved progress display while running `cargo features prune`
* `cargo features prune` now displays which features get disabled
* add list of known features to ignore when running `cargo features prune`

## 0.7.1

* make `*` as a version be a wildcard for `any` - always find prerelease versions
* handle dependency renames via `package = ""`

## 0.7.0

* workspace dependency support
* highlight empty packages
* search for packages
* support custom targets
* fix bug where it could not differentiate between dependencies

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