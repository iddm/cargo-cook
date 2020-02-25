#[cfg(feature = "bzip2")]
extern crate bzip2;
extern crate tar;
extern crate term;

use std::collections::HashMap;

/// A file name and its' content as string.
pub type File = (String, String);
pub type Files = Vec<(String, String)>;

lazy_static! {
    static ref CONTAINERS: HashMap<&'static str, fn(&str, &[File])> = {
        let mut m = HashMap::new();
        m.insert("tar", tar as fn(&str, &[File]));
        #[cfg(feature = "bzip2")]
        m.insert("tar.bzip2", bzip2 as fn(&str, &[File]));
        m
    };
}

#[cfg(feature = "bzip2")]
fn bzip2(destination_file_path: &str, files: &[File]) {
    use self::bzip2::read::BzEncoder;
    use self::bzip2::Compression;
    use std::fs::File;
    use std::io::{Read, Write};
    use term_print::*;

    const TEMP_FILE: &str = "/tmp/cooked.tar";
    const BZIP2_LABEL: &str = "[bzip2]";

    tar(TEMP_FILE, files);
    let mut tar_file = File::open(TEMP_FILE).unwrap();
    let mut raw_bytes = Vec::new();
    tar_file.read_to_end(&mut raw_bytes).unwrap();
    let mut compressed_bytes = Vec::new();
    let mut compressor = BzEncoder::new(raw_bytes.as_slice(), Compression::Best);
    compressor.read_to_end(&mut compressed_bytes).unwrap();
    let ratio = 100f32 / (raw_bytes.len() as f32 / compressed_bytes.len() as f32);
    let mut compressed_archive = File::create(destination_file_path).unwrap();
    term_println(
        self::term::color::WHITE,
        BZIP2_LABEL,
        &format!("Compressed ratio: {:.2}%", ratio),
    );
    compressed_archive
        .write_all(compressed_bytes.as_slice())
        .unwrap();
}

fn tar(destination_file_path: &str, files: &[File]) {
    use self::tar::Builder;
    use std::fs::File;

    let file = File::create(destination_file_path).unwrap();
    let mut ar = Builder::new(file);
    for f in files {
        if let Ok(mut dest_file) = File::open(&f.1) {
            ar.append_file(&f.0, &mut dest_file).unwrap();
        } else {
            panic!("No such file or directory: {}", f.1);
        }
    }
}

pub fn support_container(container: &str) -> bool {
    CONTAINERS.get::<str>(container).is_some()
}

pub fn compress(files: &[File], destination_file_path: &str, container: &str) {
    CONTAINERS.get::<str>(container).unwrap()(destination_file_path, files)
}
