#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct CookIngredient {
    pub source: String,
    pub filter: Option<String>,
    pub destination: String,
}

#[cfg(feature = "ssh")]
#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct SshConfig {
    pub hostname: String,
    pub username: String,
    pub remote_path: String,
    pub deploy_script: Option<String>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct FsCopy {
    pub path: String,
}

#[cfg(feature = "deploy")]
#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct Deploy {
    pub targets: Option<Vec<String>>,
    #[cfg(feature = "ssh")]
    pub ssh: Option<SshConfig>,
    pub fscopy: Option<FsCopy>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct Cook {
    pub target_directory: String,
    pub target_rename: Option<String>,
    pub hashes: Option<Vec<String>>,
    pub containers: Vec<String>,
    pub pre_cook: Option<String>,
    pub post_cook: Option<String>,
    pub include_dependencies: Option<bool>,
    pub cook_directory: String,
    #[cfg(feature = "deploy")]
    pub deploy: Option<Deploy>,
    pub ingredient: Option<Vec<CookIngredient>>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct CookConfig {
    pub cook: Cook,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
pub struct CargoConfig {
    pub package: Package,
}
