# Cargo Features Manger

A TUI like cli tool to manage the features of your rust-project dependencies. 

## install

`cargo install cargo-features-manager`

## usage

To start the tool run `cargo features` in your project root dir.

This will open the dependency-selector:

![dependencySelector](resources/dependencySelector.png)

Now you can select the dependency for which you want to change the enabled features.

Selecting a dependency will open the feature-selector:

![featureSelector](resources/featureSelector.png)

When using `cargo features -d <dependency name>` it will directly open the corresponding feature-selector.

### navigation

<kbd>↑</kbd> to move up

<kbd>↓</kbd> to move down

<kbd>q</kbd> to quit

<kbd>Space</kbd> | <kbd>Enter</kbd> to select

<kbd>BackSpace</kbd> to move back

### dependency selector

Dependency which do not have any features are marked grey.

![greyDependency](resources/greyDependency.png)

### feature selector

All default features are marked Green.

![greenMark](resources/greenMark.png)

When hovering above a feature it shows other features which the selected feature requires.

![featureDependency](resources/featureDependency.png)

Features which an active feature requires are marked grey.

![greyFeature](resources/greyFeature.png)
