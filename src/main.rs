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

use clap::{ App, AppSettings, SubCommand };
use regex::Regex;

use std::fs::File;
use std::io::{ Read, Write };
use std::path::Path;
use std::process::Command;
use std::panic;

const CONFIG_FILE_NAME: &'static str = "Cargo.toml";
const COMMAND_NAME: &'static str = "cook";
const COMMAND_DESCRIPTION: &'static str = "A third-party cargo extension which cooks your crate.";

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
struct CookIngredient {
    source: String,
    destination: String,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
struct Cook {
    target_directory: String,
    target_rename: Option<String>,
    hashes: Option<Vec<String>>,
    containers: Vec<String>,
    pre_cook: Option<String>,
    post_cook: Option<String>,
    include_dependencies: Option<bool>,
    cook_directory: String,
    ingredient: Option<Vec<CookIngredient>>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
struct Package {
    name: String,
    version: String,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
struct CookConfig {
    package: Package,
    cook: Cook,
}

#[derive(Default, Debug, Clone)]
struct InternalConfig {
    ingredients: Vec<(Regex, String)>, // Source regexp, destination path
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        // println!("{:?}", panic_info.payload().downcast_ref::<String>().unwrap());
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

fn term_print(color: term::color::Color, status_text: &str, text: &str) {
    let mut t = term::stdout().unwrap();

    t.attr(term::Attr::Bold).unwrap();
    t.fg(color).unwrap();
    write!(t, "{} ", status_text).unwrap();
    let _ = t.reset();
    write!(t, "{}\n", text).unwrap();
}

fn term_panic(text: &str) {
    term_print(term::color::BRIGHT_RED, "Failure:", text);
}

fn cook() {
    let config = load_config();
    #[cfg(debug_assertions)]
    println!("Config: {:?}", config);
    let internal_config = parse_config(&config);
    let pkg_name = &format!("{} v{}", config.package.name, config.package.version);
    term_print(term::color::BRIGHT_GREEN, "Cooking", pkg_name);
    cook_hook(&config.cook, true);

    archive(&config, collect(&config, &internal_config));
    upload(&config);

    cook_hook(&config.cook, false);
    term_print(term::color::BRIGHT_GREEN, "Finished", "cooking");
}


// TODO iterate over all of ingredients, return source -> destination (parse regexp)
fn collect(c: &CookConfig, ic: &InternalConfig) -> container::Files {
    let mut files = container::Files::new();
    // for (s, d) in ic.ingredients {
    // }
    
    let target_file_name = format!("{}/{}", c.cook.target_directory, c.package.name);
    let renamed_target_file_name = if let Some(s) = c.cook.target_rename.clone() { s }
                                   else { c.package.name.clone() };
    files.push((renamed_target_file_name, target_file_name));
    files
}

fn archive(c: &CookConfig, cf: container::Files) {
    std::fs::create_dir_all(&c.cook.cook_directory).unwrap();

    for cont in &c.cook.containers {
        let file_name = &format!("{}/{}-{}",
                                 c.cook.cook_directory,
                                 c.package.name,
                                 c.package.version);
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
        term_print(term::color::BRIGHT_GREEN, "Cooked", &format!("{}", archive_file_path.display()));
    }
}

// TODO implement uploading the cooked archives: filesystem, ssh, git, ftp, http, etc
fn upload(c: &CookConfig) {
    // term_print(term::color::BRIGHT_GREEN, "Uploading", "the crate.");
}

fn load_config() -> CookConfig {
    let mut config_toml = String::new();
    let config_file_name = CONFIG_FILE_NAME.to_owned();
    if let Ok(mut file) = File::open(config_file_name) {
        if let Err(e) = file.read_to_string(&mut config_toml) {
            panic!("Unable to read {}: {}", CONFIG_FILE_NAME, e);
        }
    } else {
        panic!("{} file was not found.", CONFIG_FILE_NAME);
    }
    let parsed = toml::de::from_str::<CookConfig>(&config_toml);
    if let Ok(c) = parsed {
        return c
    } else {
        panic!("Unable to parse {}: {}", CONFIG_FILE_NAME, parsed.unwrap_err());
    }
}

fn cook_hook(c: &Cook, pre: bool) -> bool {
    let hook = if pre { &c.pre_cook } else { &c.post_cook };
    let hook_name = if pre { "Pre-cook" } else { "Post-cook" };

    if let Some(ref pre) = *hook {
        term_print(term::color::YELLOW, "Executing", hook_name);
        let res = Command::new(Path::new(pre).canonicalize().unwrap()).status();
        if let Ok(s) = res {
            if s.success() {
                term_print(term::color::BRIGHT_GREEN, hook_name, &format!("returned {}",
                                                                          s.code().unwrap_or(0i32)));
            } else {
                term_print(term::color::BRIGHT_RED, hook_name, &format!("returned {}",
                                                                        s.code().unwrap_or(0i32)));
            }
            return s.success()
        } else {
            panic!("{} failed: {}", hook_name, res.unwrap_err());
        }
    }

    return true
}

fn parse_config(c: &CookConfig) -> InternalConfig {
    let mut ic = InternalConfig::default();

    for cont in &c.cook.containers {
        if !container::support_container(cont) {
            panic!("The {} container type is unsupported.", cont);
        }
    }

    if let Some(ref hashes) = c.cook.hashes {
        for h in hashes {
            if !hash::support_hash_type(h) {
                panic!("The {} hash type is unsupported.", h);
            }
        }
    }

    if let Some(ref is) = c.cook.ingredient {
        for i in is {
            ic.ingredients.push((Regex::new(&i.source).unwrap(), i.destination.clone()));
        }
    }

    ic
}
