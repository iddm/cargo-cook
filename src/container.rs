extern crate tar;

use std::fs::File;
use std::collections::HashMap;

pub type Files = Vec<(String, String)>;

lazy_static! {
    static ref CONTAINERS: HashMap<&'static str, fn(&str, &Files)> = {
        let mut m = HashMap::new();
        m.insert("tar", tar as fn(&str, &Files));
        m
    };
}

fn tar(destination_file_path: &str, files: &Files) {
    use self::tar::Builder;

    let file = File::create(destination_file_path).unwrap();
    let mut ar = Builder::new(file);
    for f in files {
        ar.append_file(&f.0,
                       &mut File::open(&f.1).unwrap()).unwrap();
    }
}

pub fn support_container(container: &str) -> bool {
    CONTAINERS.get::<str>(container).is_some()
}

pub fn compress(files: &Files, destination_file_path: &str, container: &str) {
    CONTAINERS.get::<str>(container).unwrap()(destination_file_path, files)
}
