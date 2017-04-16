# `cargo cook`

[![](https://meritbadge.herokuapp.com/cargo-cook)](https://crates.io/crates/cargo-cook) [![](https://travis-ci.org/vityafx/cargo-cook.svg?branch=master)](https://travis-ci.org/vityafx/cargo-cook)

A third-party cargo extension which lets you cook your crate. What it does:

1. Collects all the files you specified (ingredients) as same as crate's artifact (binary or a library).
2. Puts everything from `.1` into a container with possible compression.
3. Calculates hash-sums for each of container from `.2`.
4. Uploads all the files from `.3` into desired location.

If you still have not understood what is it then just read this:

> After building a crate you have a produced binary file (or a library). You may want to upload it somewhere for further downloading and use. For example, you have made a game called `rustquake` in pure Rust. You want to release it so you compile it in release mode with all optimizations and then you may want to upload the binary (and some other files like config files, libraries, images, shaders, etc) somewhere. It would also be nice to compress all of this. So, you manually create an archive, for example, `rustquake-0.1.1.tar.gz` where you manually put all the files you need. Then you go to your server, put the archive in some folder which your web-server knows about manually again. This is a lot of routine work. But all ot these steps may be performed automatically by the `cargo cook`.

# Configuring

To make it work with your crate you must create a file `Cook.toml` in the root directory of your crate.
Let's look at the [`Cook.toml.example`](https://github.com/vityafx/cargo-cook/blob/master/Cook.toml.example) of `cargo-cook` crate:

```toml
[cook]
target_directory = "target/release"
target_rename = "cargocook"
hashes = ["md5", "sha256", "sha512"]
containers = ["tar", "tar.bzip2"]
pre_cook = "pre_cook.sh"
post_cook = "post_cook.sh"
include_dependencies = true
cook_directory = "cooked/"

[cook.deploy]
targets = ["fscopy", "ssh"]

[cook.deploy.fscopy]
path = "/tmp"

[cook.deploy.ssh]
hostname = ""                           # Host:port format.
username = ""
remote_path = ""                        # must be absolute path!
deploy_script = "ssh_deploy.sh"         # Will be executed on remote server.


# If source is a file then it will be copied to the destination.
# If the source is a directory then the destination field is also a directory and `filter` field can be used to determine which files to take.
[[cook.ingredient]]
source = "Cargo.toml"
destination = "Cargo.toml"

[[cook.ingredient]]
source = "src"
destination = "src"

[[cook.ingredient]]
source = "./"
filter = "(LICENSE-*)"
destination = "licenses/"
```

**Cook**
- `target_directory` - a directory where to find your crate artifacts.
- `target_rename` **(Optional)** - rename the target file before packaging into a container.
- `hashes` **(Optional)** - a list of hash-sum algorithms which will be used for calculating hashsumm of the containers.
- `containers` - a list of containers into which your ingredients will be packed.
- `pre_cook` **(Optional)** - a script which will be executed before cooking.
- `post_cook` **(Optional)** - a script which will be executed after cooking.
- `include_dependencies` **(Optional)** - include crate dependencies into the container.
- `cook_directory` - a directory where containers will be put.

**Deploy**
- `targets` - a list of deploy targets.

**deploy.fscopy**
- `path` - a string where to copy cooked files.

**deploy.ssh**
- `hostname` - a string with hostname and port (`github.com:80` for example).
- `username` - a string with username which will be used for deploying. Password will be asked during deployment.
- `remote_path` - a string which points to a remote path where cooked files will be copied.
- `deploy_script` **(Optional)** - a string which will be executed on the remote server with `remote_path` as working directory.

**Ingredient**
- `source` - a string which is a path to file or a directory. If it is a directory then `filter` field may be used.
- `filter` **(Optional)** - a regular expression which will be used to determine the ingredients.
- `destination` - a string which is a path to file or a directory. If `source` is a file then `destination` is also a file, otherwise it is a directory where files from `source` directory will be put.

So, if you will just perform `cargo cook` in the directory with the `cargo cook` crate with the configuration described above it will give you:

```bash
$ cd cargo-cook

$ cargo cook
  Cooking cargo-cook v0.1.5
  Executing Pre-cook
  Hello from pre_cook.sh
  Pre-cook returned 0
  Cooked /home/workspace/cargo-cook/cooked/cargo-cook-0.1.5.tar
  Executing Post-cook
  Hello from post_cook.sh
  Post-cook returned 0
  Finished cooking

$ ls cooked/
  cargo-cook-0.1.5.tar
  cargo-cook-0.1.5.tar.md5
  cargo-cook-0.1.5.tar.sha256
  cargo-cook-0.1.5.tar.sha512

$ tar -xvf cargo-cook-0.1.5.tar
  Cargo.toml
  src/main.rs
  src/container.rs
  src/hash.rs
  licenses/LICENSE-APACHE
  licenses/LICENSE-MIT
  cargocook
```

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
