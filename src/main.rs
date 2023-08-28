use std::{fmt::Display, process};

use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use tokio;

use crate::conf::{Args, Config, DotDesktop};

#[tokio::main]
async fn main() {
    let args = Args::from_args();

    if !DotDesktop::exists() {
        DotDesktop::create(&args.dest_dir);
    }
    let mut conf = if let Some(conf) = Config::load() {
        conf
    } else {
        Config::new(args.dest_dir, Version::default())
    };

    let client = req::Client::new();
    let latest_version = match client.get_latest_release_number().await {
        Ok(version_number_parse) => match version_number_parse {
            Some(version_number) => version_number,
            None => {
                eprintln!("Could not parse version numbers");
                process::exit(-1);
            }
        },
        Err(e) => {
            eprintln!("Could not access thunderbird website {:?}", e);
            process::exit(-1);
        }
    };

    if latest_version <= conf.version {
        println!(
            "Latest release is {} and we already have {}",
            &latest_version, &conf.version
        );
        process::exit(0);
    }
    println!("Downloading version {}", &latest_version);

    let status = client
        .download_version(&latest_version, &conf.dest_dir)
        .await;

    match status {
        Ok(_) => println!("Successfully updated thunderbird, please restart it"),
        Err(e) => {
            eprintln!("Encountered the following error {:?}", e);
            process::exit(-1);
        }
    }

    conf.version = latest_version;
    if let Err(e) = conf.save() {
        eprintln!("Failed to save config/state {:?}", e);
    }
}

mod conf {
    use std::{
        env,
        fs::{self, File},
        io::{BufReader, BufWriter},
        str,
    };

    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use structopt::StructOpt;

    use crate::Version;

    const DOT_DESKTOP_PATH: &'static str = "~/.local/share/applications/thunderbird.desktop";

    pub struct DotDesktop;
    impl DotDesktop {
        pub fn create(thunderbird_dest: &str) {
            let dot_desktop = include_bytes!("thunderbird.desktop");
            let home = env::var("HOME").unwrap();
            let dot_desktop = str::from_utf8(dot_desktop)
                .expect("thunderbird.desktop wasn't UTF8")
                .replace("PLACEHOLDER", &format!("{}/{}", home, thunderbird_dest));
            if let Err(_) = fs::write(DOT_DESKTOP_PATH.replace("~", &home), dot_desktop.as_bytes())
            {
                eprintln!("Failed to write .desktop to {}", DOT_DESKTOP_PATH);
            }
        }

        pub fn exists() -> bool {
            fs::metadata(DOT_DESKTOP_PATH).is_ok()
        }
    }

    const CONFIG_PATH: &'static str = "~/.config/tb-updater.json";

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub version: Version,
        pub dest_dir: Box<str>,
    }
    impl Config {
        pub fn new(dest_dir: impl Into<Box<str>>, version: Version) -> Self {
            Self {
                version,
                dest_dir: dest_dir.into(),
            }
        }

        pub fn load() -> Option<Self> {
            let reader = BufReader::new(File::open(CONFIG_PATH).ok()?);
            serde_json::from_reader(reader).ok()
        }

        pub fn save(&self) -> Result<()> {
            let mut writer = BufWriter::new(File::create(CONFIG_PATH)?);
            serde_json::to_writer(&mut writer, &self)?;
            Ok(())
        }
    }

    #[derive(StructOpt)]
    pub struct Args {
        #[structopt(short = "d", long = "dest-dir", default_value = "~/Downloads")]
        pub dest_dir: String,
    }
}

mod req {
    use bzip2::read::BzDecoder;
    use http::header;
    use indicatif::{ProgressBar, ProgressStyle};
    use scraper::{Html, Selector};
    use std::{collections::VecDeque, io::Cursor};
    use tar::Archive;

    use crate::Version;

    const RELEASES: &'static str = "https://www.thunderbird.net/en-US/thunderbird/releases/";
    pub struct Client {
        client: reqwest::Client,
    }
    impl Client {
        pub fn new() -> Self {
            Self {
                client: reqwest::Client::new(),
            }
        }

        pub async fn get_content(&self) -> Result<String, reqwest::Error> {
            self.client.get(RELEASES).send().await?.text().await
        }

        pub async fn get_latest_release_number(&self) -> anyhow::Result<Option<Version>> {
            let content = self.get_content().await?;
            let fragment = Html::parse_fragment(&content);
            let version_selector = Selector::parse("a.inline-link").unwrap();

            let mut versions = fragment
                .select(&version_selector)
                .filter_map(|element| element.text().next())
                .filter_map(Version::from_str)
                .collect::<VecDeque<_>>();

            versions.make_contiguous().sort();
            let a = versions.pop_back();
            Ok(a)
        }

        pub async fn download_version(
            &self,
            version: &Version,
            dest_dir: &str,
        ) -> anyhow::Result<()> {
            let url = format!("https://download-installer.cdn.mozilla.net/pub/thunderbird/releases/{0}/linux-x86_64/en-US/thunderbird-{0}.tar.bz2", &version);
            let mut resp = self.client.get(&url).send().await?;

            let total_size = resp
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|ct_len| ct_len.to_str().ok())
                .and_then(|ct_len| ct_len.parse::<u64>().ok())
                .unwrap_or(0);

            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));

            let mut writer = Vec::new();
            while let Some(chunk) = resp.chunk().await? {
                pb.inc(chunk.len() as u64);
                writer.extend_from_slice(&chunk);
            }

            pb.finish_with_message("Download complete.");

            let decoder = BzDecoder::new(Cursor::new(writer));
            let mut archive = Archive::new(decoder);
            archive.unpack(dest_dir)?;

            Ok(())
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize, Default)]
pub struct Version {
    major: i32,
    minor: i32,
    patch: i32,
}

impl Version {
    fn from_str(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}