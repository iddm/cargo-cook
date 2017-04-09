#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate regex;
extern crate tar;
extern crate crypto;
extern crate term;

use clap::{ App, AppSettings, SubCommand };
use regex::Regex;

use std::fs::File;
use std::io::{ Read, Write };
use std::path::Path;
use std::process::Command;
use std::collections::BTreeMap;
use std::panic;

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
    t.reset();
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

    archive(&config, collect(&internal_config));
    upload(&config);

    cook_hook(&config.cook, false);
    term_print(term::color::BRIGHT_GREEN, "Finished", "cooking");
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

    std::fs::create_dir_all(&c.cook.cook_directory).unwrap();

    let file_name = &format!("{}/{}-{}",
                             c.cook.cook_directory,
                             c.package.name,
                             c.package.version);
    let archive_file_name = &format!("{}.tar", file_name);
    let hash_file_name = &format!("{}.md5", archive_file_name);
    let target_file_name = &format!("{}/{}", c.cook.target_directory, c.package.name);
    let renamed_target_file_name = if let Some(ref s) = c.cook.target_rename { s }
                                   else { &c.package.name };
    // Archive
    {
        let file = File::create(archive_file_name).unwrap();
        let mut ar = Builder::new(file);
        ar.append_file(renamed_target_file_name,
                       &mut File::open(target_file_name).unwrap()).unwrap();
    }

    // Hash
    let mut hash = Md5::new();
    let mut file_bytes = Vec::new();
    let mut f = File::open(archive_file_name).unwrap();
    f.read_to_end(&mut file_bytes);
    hash.input(file_bytes.as_slice());
    f = File::create(hash_file_name).unwrap();
    writeln!(f, "{}", hash.result_str());
    let archive_file_path = Path::new(archive_file_name).canonicalize().unwrap();
    term_print(term::color::BRIGHT_GREEN, "Cooked", &format!("{}", archive_file_path.display()));
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
