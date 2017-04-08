# `cargo cook`

[![Build Status](https://travis-ci.org/vityafx/cargo-cook.svg?branch=master)](https://travis-ci.org/vityafx/cargo-cook)

A third-party cargo extension to allow you to cook your crate.

# Compiling

Assuming you already have Rust and cargo set up.

Clone this repository and go into the created directory:

    git clone https://github.com/vityafx/cargo-cook.git
    cd cargo-cook

And compile a release version:

    cargo build --release

You should now have an executable in `[starting directory]/cargo-cook/target/release/cargo-cook`.

# Installing and Using

Compile the code as shown in the previous section, then put the `cargo-cook` executable in your PATH.

My favorite way of doing this is I have a pre-existing directory in `~/bin` that contains little scripts of mine, that dir is added to my PATH in my `.bashrc` so that it's always available, and then I symlink the release version from where it exists to that directory:

    ln -s [starting directory]/cargo-cook/target/release/cargo-cook ~/bin/

Once you've done that, because of the way cargo is set up to use third party extensions, in any other Rust project of yours, you should be able to run:

    cargo cook

and that crate will be cooked.

# Contributing

If you'd like to work on your own version of the code, fork this repo and follow the compiling steps above except with your fork.

One weird thing if you're running the binary directly instead of through the `cargo` plugin system is that clap doesn't think you're using a subcommand. If you try, you'll get:

    $ ./target/release/cargo-cook whatever
    error: Found argument 'whatever', but cargo wasn't expecting any

    USAGE:
            cargo <SUBCOMMAND>

    For more information try --help

To get around this, either follow the Installation and Usage instructions above and always use `cargo cook whatever` or re-specify `cook` as the subcommand:

    ./target/release/cargo-cook cook whatever

# License

`cargo cook` is primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See LICENSE-APACHE and LICENSE-MIT for details.
