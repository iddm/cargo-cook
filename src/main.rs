#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
extern crate serde;
extern crate toml;
extern crate regex;
extern crate term;

mod hash;
mod container;
#[cfg(feature = "deploy")]
mod deploy;
mod config;
mod term_print;

use clap::{ App, AppSettings, SubCommand };
use regex::Regex;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
#[cfg(not(debug_assertions))]
use std::panic;

use config::*;
use term_print::*;


const CONFIG_FILE_NAME: &'static str = "Cook.toml";
const CARGO_TOML: &'static str = "Cargo.toml";
const COMMAND_NAME: &'static str = "cook";
const COMMAND_DESCRIPTION: &'static str = "A third-party cargo extension which cooks your crate.";


fn main() {
    #[cfg(not(debug_assertions))]
    panic::set_hook(Box::new(|panic_info| {
        term_panic(panic_info.payload().downcast_ref::<String>().unwrap());
    }));

    let _ = App::new(format!("cargo-{}", COMMAND_NAME))
        .about(COMMAND_DESCRIPTION)
        .version(&crate_version!()[..])
        // We have to lie about our binary name since this will be a third party
        // subcommand for cargo, this trick learned from cargo-outdated
        .bin_name("cargo")
        // We use a subcommand because parsed after `cargo` is sent to the third party plugin
        // which will be interpreted as a subcommand/positional arg by clap
        .subcommand(SubCommand::with_name(COMMAND_NAME)
            .about(COMMAND_DESCRIPTION))
        .settings(&[AppSettings::SubcommandRequired])
        .get_matches();

    cook();
}

fn cook() {
    let cook_config = load_config::<CookConfig>(CONFIG_FILE_NAME);
    let cargo_config = load_config::<CargoConfig>(CARGO_TOML);
    #[cfg(debug_assertions)]
    println!("Config: {:?}", cook_config);
    parse_config(&cook_config);
    let pkg_name = &format!("{} v{}", cargo_config.package.name, cargo_config.package.version);
    term_println(term::color::BRIGHT_GREEN, "Cooking", pkg_name);
    cook_hook(&cook_config.cook, true);

    archive(&cook_config, &cargo_config, collect(&cook_config, &cargo_config));
    #[cfg(feature = "deploy")]
    deploy(&cook_config);

    cook_hook(&cook_config.cook, false);
    term_println(term::color::BRIGHT_GREEN, "Finished", "cooking");
}

fn collect_recursively(source: &str, destination: &str, files: &mut container::Files) {
    use std::fs;

    let path = Path::new(source);
    if !path.is_dir() {
        panic!("{} is not a directory!", path.display());
    }
    for entry in fs::read_dir(path).unwrap() {
        let e = entry.unwrap();
        let name = e.file_name().into_string().unwrap();
        files.push((format!("{}/{}", destination.to_owned(), name),
                    e.path().to_str().unwrap().to_owned()));
    }
}

fn collect(c: &CookConfig, cargo: &CargoConfig) -> container::Files {
    use std::fs;

    let mut files = container::Files::new();
    if let Some(ref ingredients) = c.cook.ingredient {
        for i in ingredients {
            let path = Path::new(&i.source);
            if path.is_file() {
                files.push((i.source.clone(), i.destination.clone()));
            } else if path.is_dir() {
                if let Some(ref filter) = i.filter {
                    let r = Regex::new(filter).unwrap();
                    for entry in fs::read_dir(path).unwrap() {
                        let e = entry.unwrap();
                        let name = e.file_name().into_string().unwrap();
                        if r.is_match(&name) {
                            files.push((format!("{}/{}", i.destination.clone(), name),
                                        e.path().to_str().unwrap().to_owned()));
                        }
                    }
                } else {
                    collect_recursively(&i.source, &i.destination, &mut files);
                }
            } else {
                panic!("Specified ingredient ({}) is neither a file nor a directory.", i.source);
            }
        }
    }
    
    let target_file_name = format!("{}/{}", c.cook.target_directory, cargo.package.name);
    let renamed_target_file_name = if let Some(s) = c.cook.target_rename.clone() { s }
                                   else { cargo.package.name.clone() };
    files.push((renamed_target_file_name, target_file_name));
    files
}

fn archive(c: &CookConfig, cargo: &CargoConfig, cf: container::Files) {
    std::fs::create_dir_all(&c.cook.cook_directory).unwrap();

    for cont in &c.cook.containers {
        let file_name = &format!("{}/{}-{}",
                                 c.cook.cook_directory,
                                 cargo.package.name,
                                 cargo.package.version);
        let archive_file_name = &format!("{}.{}", file_name, cont);
        // Archive
        container::compress(&cf, archive_file_name, cont);

        // Hash
        if let Some(ref hashes) = c.cook.hashes {
            for hash_type in hashes {
                let hash_file_name = &format!("{}.{}", archive_file_name, hash_type);
                hash::write_file_hash(archive_file_name, hash_file_name, hash_type);
            }
        }

        let archive_file_path = Path::new(archive_file_name).canonicalize().unwrap();
        term_println(term::color::BRIGHT_GREEN, "Cooked", &format!("{}", archive_file_path.display()));
    }
}

// TODO implement uploading the cooked archives: filesystem, ssh, git, ftp, http, etc
#[cfg(feature = "deploy")]
fn deploy(c: &CookConfig) {
    if let Some(ref deploy) = c.cook.deploy {
        if let Some(ref targets) = deploy.targets {
            term_println(term::color::BRIGHT_GREEN, "Deploying", "the crate.");

            for t in targets {
                let target_str = format!("[{}]", t);
                if let Err(e) = deploy::deploy(t, &c.cook.cook_directory, deploy) {
                    term_println(term::color::BRIGHT_RED, &target_str, &e);
                } else {
                    term_println(term::color::BRIGHT_GREEN, &target_str, "OK");
                }
            }
        }
    }
}

fn load_config<T: std::fmt::Debug + serde::Deserialize>(file_name: &str) -> T {
    let mut config_toml = String::new();
    let config_file_name = file_name.to_owned();
    if let Ok(mut file) = File::open(config_file_name.clone()) {
        if let Err(e) = file.read_to_string(&mut config_toml) {
            panic!("Unable to read {}: {}", config_file_name, e);
        }
    } else {
        panic!("{} file was not found.", config_file_name);
    }
    let parsed = toml::de::from_str::<T>(&config_toml);
    if let Ok(c) = parsed {
        return c
    } else {
        panic!("Unable to parse {}: {}", config_file_name, parsed.unwrap_err());
    }
}

fn cook_hook(c: &Cook, pre: bool) -> bool {
    let hook = if pre { &c.pre_cook } else { &c.post_cook };
    let hook_name = if pre { "Pre-cook" } else { "Post-cook" };

    if let Some(ref pre) = *hook {
        term_println(term::color::YELLOW, "Executing", hook_name);
        let res = Command::new(Path::new(pre).canonicalize().unwrap()).status();
        if let Ok(s) = res {
            if s.success() {
                term_println(term::color::BRIGHT_GREEN, hook_name, &format!("returned {}",
                                                                            s.code().unwrap_or(0i32)));
            } else {
                term_println(term::color::BRIGHT_RED, hook_name, &format!("returned {}",
                                                                          s.code().unwrap_or(0i32)));
            }
            return s.success()
        } else {
            panic!("{} failed: {}", hook_name, res.unwrap_err());
        }
    }

    return true
}

fn parse_config(c: &CookConfig) {
    for cont in &c.cook.containers {
        if !container::support_container(cont) {
            panic!("The \"{}\" container type is unsupported.", cont);
        }
    }

    if let Some(ref hashes) = c.cook.hashes {
        for h in hashes {
            if !hash::support_hash_type(h) {
                panic!("The \"{}\" hash type is unsupported.", h);
            }
        }
    }

    #[cfg(feature = "deploy")]
    let check_deploy = |c: &CookConfig| {
        if let Some(ref deploy) = c.cook.deploy {
            if let Some(ref targets) = deploy.targets {
                for t in targets {
                    if !deploy::support_deploy_target(t) {
                        panic!("The \"{}\" deploy target is unsupported.", t);
                    }
                }
            }
        }
    };

    #[cfg(feature = "deploy")]
    check_deploy(&c);
}
