#[cfg(feature = "ssh")]
extern crate rpassword;
#[cfg(feature = "ssh")]
extern crate ssh2;
extern crate term;

use config::Deploy;
use std::collections::HashMap;
use std::result::Result as StdResult;

pub type Result = StdResult<(), String>;

lazy_static! {
    static ref TARGETS: HashMap<&'static str, fn(&str, &Deploy) -> Result> = {
        let mut m = HashMap::new();
        #[cfg(feature = "ssh")]
        m.insert("ssh", ssh as fn(&str, &Deploy) -> Result);
        m.insert("fscopy", fscopy as fn(&str, &Deploy) -> Result);
        m
    };
}

fn fscopy(source: &str, d: &Deploy) -> Result {
    use std::fs;
    use std::path::Path;
    use term_print::*;

    const FSCOPY_LABEL: &'static str = "[fscopy]";

    if let Some(ref fscopy) = d.fscopy {
        let path = Path::new(source);
        let dir = fs::read_dir(path).unwrap();
        for entry in dir {
            let e = entry.unwrap();
            let entry_path = e.path();
            let path = entry_path.to_str().unwrap();
            if let Some(file_name) = e.file_name().to_str() {
                term_rprint(
                    self::term::color::WHITE,
                    FSCOPY_LABEL,
                    &format!("Copying \"{}\" to \"{}\"", path, fscopy.path),
                );
                if let Err(err) = fs::copy(e.path(), &format!("{}/{}", fscopy.path, file_name)) {
                    term_rprint_finish();
                    return Err(err.to_string());
                }
                term_rprint(
                    self::term::color::WHITE,
                    FSCOPY_LABEL,
                    &format!("Copied \"{}\" to \"{}\"", path, fscopy.path),
                );
                term_rprint_finish();
            }
        }
    }

    Ok(())
}

#[cfg(feature = "ssh")]
fn ssh(source: &str, d: &Deploy) -> Result {
    use self::rpassword::read_password;
    use self::ssh2::Session;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use term_print::*;

    const SSH_LABEL: &'static str = "[ssh]";

    let exec = |sess: &Session, cmd: &str| {
        let channel_session = sess.channel_session();
        if let Ok(mut channel) = channel_session {
            let res = channel.exec(cmd);
            if let Err(e) = res {
                term_println(
                    self::term::color::RED,
                    SSH_LABEL,
                    &format!("Failed to execute command: {}", e),
                );
            } else {
                let mut s = String::new();
                if let Ok(_) = channel.read_to_string(&mut s) {
                    if !s.is_empty() {
                        term_print(
                            self::term::color::WHITE,
                            &format!("{} ({}):", SSH_LABEL, cmd),
                            &s,
                        );
                    }
                }
            }
        } else {
            term_println(
                self::term::color::RED,
                SSH_LABEL,
                "Failed to get channel for command execution.",
            );
        }
    };

    let send_file = |sess: &Session, local_path: &Path, remote_path: &Path| {
        let mut buffer = vec![];
        let mut read = 0u64;
        if let Ok(mut file) = File::open(local_path) {
            let metadata = file.metadata().unwrap();
            let file_size = metadata.len();
            let file_path_str = local_path.to_str().unwrap();
            let mut remote_file = sess
                .scp_send(
                    remote_path,
                    metadata.permissions().mode() as i32,
                    file_size,
                    None,
                ).unwrap();
            while let Ok(read_bytes) = file.read(&mut buffer) {
                if read_bytes == 0usize {
                    break;
                }
                read += read_bytes as u64;
                remote_file.write_all(&buffer).unwrap();
                term_rprint(
                    self::term::color::WHITE,
                    SSH_LABEL,
                    &format!(
                        "Sending \"{}\" [{:.2} MB of {:.2} MB]",
                        file_path_str,
                        read as f64 / 1048576.0f64,
                        file_size as f64 / 1048576.0f64
                    ),
                );
            }
            term_rprint_finish();
        }
    };

    if let Some(ref ssh) = d.ssh {
        term_println(
            self::term::color::WHITE,
            SSH_LABEL,
            &format!("Connecting to {}", ssh.hostname),
        );
        let tcp = TcpStream::connect(&ssh.hostname).unwrap();
        let mut sess = Session::new().unwrap();
        sess.handshake(&tcp).unwrap();

        for i in 0..3 {
            term_print(
                self::term::color::WHITE,
                SSH_LABEL,
                &format!("Password for {}: ", ssh.username),
            );
            let ssh_password = read_password().unwrap();

            if ssh_password.is_empty() {
                if i == 2 {
                    return Err("SSH password can not be empty.".to_owned());
                } else {
                    term_println(
                        self::term::color::YELLOW,
                        SSH_LABEL,
                        "Password can not be empty.",
                    );
                    continue;
                }
            }

            term_println(self::term::color::WHITE, SSH_LABEL, "Authorizing...");
            if let Err(e) = sess.userauth_password(&ssh.username, &ssh_password) {
                if i == 2 {
                    return Err(e.to_string());
                } else {
                    term_println(self::term::color::RED, SSH_LABEL, &e.to_string());
                    continue;
                }
            } else {
                break;
            }
        }

        term_println(self::term::color::WHITE, SSH_LABEL, "Uploading files...");

        let path = Path::new(source);
        exec(&sess, &format!("mkdir -p {}", ssh.remote_path));
        let dir = fs::read_dir(path).unwrap();
        for entry in dir {
            let e = entry.unwrap();
            let file_name_str = e.file_name().into_string().unwrap();
            let remote_path_str = format!("{}/{}", ssh.remote_path, file_name_str);
            let remote_path = Path::new(&remote_path_str);
            send_file(&sess, &e.path(), &remote_path);
        }

        if let Some(ref ds) = ssh.deploy_script {
            term_println(
                self::term::color::WHITE,
                SSH_LABEL,
                &format!("Uploading deploy script: {}", ds),
            );

            let remote_path_str = format!("{}/{}", ssh.remote_path, ds);
            let local_path = Path::new(ds);
            let remote_path = Path::new(&remote_path_str);

            send_file(&sess, &local_path, &remote_path);

            term_println(
                self::term::color::WHITE,
                SSH_LABEL,
                &format!("Executing deploy script: {}", ds),
            );
            exec(
                &sess,
                &format!("cd {}; sh {}", ssh.remote_path, remote_path_str),
            );
            exec(&sess, &format!("rm {}", remote_path_str));
        }
    }

    Ok(())
}

pub fn support_deploy_target(target: &str) -> bool {
    TARGETS.get::<str>(&target.to_lowercase()).is_some()
}

pub fn deploy(target: &str, cook_dir: &str, d: &Deploy) -> Result {
    TARGETS.get::<str>(&target.to_lowercase()).unwrap()(cook_dir, d)
}
