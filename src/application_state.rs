/*
* BSD 2-Clause License

* Copyright (c) 2023, Love Billenius

* Redistribution and use in source and binary forms, with or without
* modification, are permitted provided that the following conditions are met:

* 1. Redistributions of source code must retain the above copyright notice, this
*    list of conditions and the following disclaimer.

* 2. Redistributions in binary form must reproduce the above copyright notice,
*    this list of conditions and the following disclaimer in the documentation
*    and/or other materials provided with the distribution.

* THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
* AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
* IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
* DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
* FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
* DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
* SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
* CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
* OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
* OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/
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

const APPLICATION_NAME: &'static str = "thunderbird.desktop";
const SHARE_APPLICATIONS_DIR: &'static str = ".local/share/applications";
const CONFIG_PATH: &'static str = ".config/tb-updater.json";

pub struct DotDesktop;
impl DotDesktop {
    pub fn create(thunderbird_dest: &str) {
        let home = env::var("HOME").unwrap();
        let local_share_path = format!("{}/{}", home, SHARE_APPLICATIONS_DIR);
        let dot_desktop_path = format!("{}/{}", &local_share_path, APPLICATION_NAME);

        if fs::metadata(&local_share_path).is_err() {
            fs::create_dir(SHARE_APPLICATIONS_DIR)
                .expect(&format!("Failed to create dir {}", local_share_path));
        }

        let dot_desktop = include_bytes!("thunderbird.desktop");
        let home = env::var("HOME").unwrap();
        let dot_desktop = str::from_utf8(dot_desktop)
            .expect("thunderbird.desktop wasn't UTF8")
            .replace("PLACEHOLDER", thunderbird_dest);
        if let Err(_) = fs::write(dot_desktop_path.replace("~", &home), dot_desktop.as_bytes()) {
            eprintln!("Failed to write .desktop to {}", &dot_desktop_path);
        }
    }

    pub fn exists() -> bool {
        let home = env::var("HOME").unwrap();
        let dot_desktop_path = format!("{}/{}/{}", home, SHARE_APPLICATIONS_DIR, APPLICATION_NAME);
        fs::metadata(dot_desktop_path).is_ok()
    }
}

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
        let home = env::var("HOME").unwrap();
        let reader = BufReader::new(File::open(format!("{}/{}", home, CONFIG_PATH)).ok()?);
        serde_json::from_reader(reader).ok()
    }

    pub fn save(&self) -> Result<()> {
        let home = env::var("HOME").unwrap();
        let mut writer = BufWriter::new(File::create(format!("{}/{}", home, CONFIG_PATH))?);
        serde_json::to_writer(&mut writer, &self)?;
        Ok(())
    }
}

#[derive(StructOpt)]
pub struct Args {
    #[structopt(short = "d", long = "dest-dir", default_value = "~/Downloads")]
    pub dest_dir: String,
}
