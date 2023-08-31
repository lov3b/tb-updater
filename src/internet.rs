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

    pub async fn download_version(&self, version: &Version, dest_dir: &str) -> anyhow::Result<()> {
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
