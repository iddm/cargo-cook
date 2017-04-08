#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate regex;
extern crate tar;
extern crate crypto;

use clap::{ App, AppSettings, SubCommand };
use regex::Regex;

use std::fs::File;
use std::io::{ Read, Write };
use std::path::Path;
use std::process::Command;
use std::collections::BTreeMap;

type CollectedFiles = BTreeMap<String, String>;

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
    println!("Your crate has been cooked successfully.");
}

fn cook() {
    let config = load_config();
    #[cfg(debug_assertions)]
    println!("Config: {:?}", config);
    let internal_config = parse_config(&config);
    cook_hook(&config.cook, true);

    archive(&config, collect(&internal_config));
    upload(&config);

    cook_hook(&config.cook, false);
}


// TODO iterate over all of ingredients, return source -> destination (parse regexp)
fn collect(c: &InternalConfig) -> CollectedFiles {
    // for (s, d) in c.ingredients {
    // }
    CollectedFiles::default()
}

// TODO iterate over `c.cook.containers` and create an archive for each of specified container.
// TODO move from `tar` crate to something common like `libarchive`.
fn archive(c: &CookConfig, cf: CollectedFiles) {
    // for c in &c.cook.containers {
    //
    // }
    use tar::Builder;
    use crypto::digest::Digest;
    use crypto::md5::Md5;

    let file_name = &format!("{}-{}", c.package.name, c.package.version);
    let archive_file_name = &format!("{}.tar", file_name);
    let hash_file_name = &format!("{}.md5", file_name);
    // Archive
    {
        let file = File::create(archive_file_name).unwrap();
        let mut ar = Builder::new(file);
        ar.append_path(&format!("{}/{}", c.cook.target_directory, c.package.name)).unwrap();
    }

    // Hash
    let mut hash = Md5::new();
    let mut file_bytes = Vec::new();
    let mut f = File::open(archive_file_name).unwrap();
    f.read_to_end(&mut file_bytes);
    hash.input(file_bytes.as_slice());
    f = File::create(hash_file_name).unwrap();
    write!(f, "{}", hash.result_str());
}

// TODO implement uploading the cooked archives: filesystem, ssh, git, ftp, http, etc
fn upload(c: &CookConfig) {
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
    let hook_name = if pre { "Pre" } else { "Post" };

    if let Some(ref pre) = *hook {
        let res = Command::new(Path::new(pre).canonicalize().unwrap()).status();
        if let Ok(s) = res {
            println!("{}-cook returned {}", hook_name, s);
            return s.success()
        } else {
            panic!("{}-cook failed: {}", hook_name, res.unwrap_err());
        }
    }

    return true
}

fn parse_config(c: &CookConfig) -> InternalConfig {
    let mut ic = InternalConfig::default();

    if let Some(ref hashes) = c.cook.hashes {
        for h in hashes {
            // TODO if we don't support specified hasher - panic!
        }
    }

    if let Some(ref is) = c.cook.ingredient {
        for i in is {
            ic.ingredients.push((Regex::new(&i.source).unwrap(), i.destination.clone()));
        }
    }

    ic
}
