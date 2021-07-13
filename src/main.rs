use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;
use toml;
use regex::Regex;

#[derive(Deserialize, Debug, PartialEq)]
struct Package {
    category: String,
    description: String,
    name: String,
    optional: bool,
    #[serde(rename(deserialize = "python-versions"))]
    python_versions: String,
    version: String,
}

#[derive(Deserialize, Debug, PartialEq)]
struct LockFile {
    package: Vec<Package>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Spec {
    op: String,
    ver: String,
}

use std::iter::Iterator;

fn parse_spec(spectxts: &str) -> Result<Vec<Spec>, String> {
    Ok(
        spectxts
        .split(",")
        .map(|item| {
            let spectxt = item.trim();

            let re = Regex::new(r"[^!=<>~]").unwrap();
            let op = re.replace_all(spectxt, "").to_string();

            let re = Regex::new(r"[^0-9\.\*]").unwrap();
            let ver = re.replace_all(spectxt, "").to_string();

            Spec { op, ver }
        })
        .collect()
    )
}

impl LockFile {
    fn to_requirements(&self, dev_mode: bool) -> String {
        self.package
            .iter()
            .filter(|p| if !dev_mode {
                p.category == "main"
            } else {
                vec![ "main", "dev" ].contains(&p.category.as_str())
            })
            .map(|p| {
                let mut depline = format!("{}=={}", p.name, p.version);
                if p.python_versions != "*" {
                    let pyvers: String = parse_spec(p.python_versions.as_str()).unwrap()
                    .iter()
                    .map(|spec| format!("python_version {} \"{}\"", spec.op, spec.ver))
                    .fold("".to_string(), |acc, pv| if acc.is_empty() { pv } else { format!("{} and {}", acc, pv) });
                    depline = format!("{}; {}", depline, pyvers);
                }
                depline
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

fn parse_lockfile(text: &str) -> Result<LockFile, ()> {
    toml::from_str::<LockFile>(text).map_err(|_| ())
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "poetry-to-requirements",
    about = "Convert Poetry.lock to requirements.txt"
)]
struct Opt {
    #[structopt(
        short = "-d", // method with no arguments - always magical
        long = "--dev", // method with one argument
    )]
    is_dev: bool,
    /// path to Poetry.lock
    #[structopt(parse(from_os_str))]
    path: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    match fs::read_to_string(opt.path)
        .map_err(|_| ())
        .and_then(|text| parse_lockfile(&text))
    {
        Ok(l) => println!("{}", l.to_requirements(opt.is_dev)),
        Err(_) => {
            eprintln!("problem parsing lockfile");
            process::exit(1);
        }
    };
}

#[test]
fn test_string_to_lockfile() {
    let lock_file_text = r#"
        [[package]]
        category = "dev"
        description = "An abstract syntax tree for Python with inference support."
        name = "astroid"
        optional = false
        python-versions = ">=3.4.*"
        version = "2.2.5"

        [package.dependencies]
        lazy-object-proxy = "*"
        six = "*"
        typed-ast = ">=1.3.0"
        wrapt = "*"
        [[package]]
        category = "dev"
        description = "Atomic file writes."
        name = "atomicwrites"
        optional = false
        python-versions = ">=2.7, !=3.0.*, !=3.1.*, !=3.2.*, !=3.3.*"
        version = "1.3.0"
    "#;

    let lock_file = parse_lockfile(lock_file_text);

    assert_eq!(
        lock_file,
        Ok(LockFile {
            package: vec![
                Package {
                    category: "dev".to_string(),
                    description: "An abstract syntax tree for Python with inference support."
                        .to_string(),
                    name: "astroid".to_string(),
                    optional: false,
                    python_versions: ">=3.4.*".to_string(),
                    version: "2.2.5".to_string()
                },
                Package {
                    category: "dev".to_string(),
                    description: "Atomic file writes.".to_string(),
                    name: "atomicwrites".to_string(),
                    optional: false,
                    python_versions: ">=2.7, !=3.0.*, !=3.1.*, !=3.2.*, !=3.3.*".to_string(),
                    version: "1.3.0".to_string()
                }
            ]
        })
    );

    assert_eq!(
        lock_file.unwrap().to_requirements(),
        "astroid==2.2.5\natomicwrites==1.3.0"
    );
}
