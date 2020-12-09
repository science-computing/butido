use std::path::PathBuf;

use indicatif::*;
use uuid::Uuid;
use url::Url;

#[derive(Clone, Debug)]
pub struct ProgressBars {
    bar_template: String,
    spinner_template: String,
    hide: bool,
}

impl ProgressBars {
    pub fn setup(bar_template: String, spinner_template: String, hide: bool) -> Self {
        ProgressBars {
            bar_template,
            spinner_template,
            hide,
        }
    }

    pub fn tree_building(&self) -> ProgressBar {
        self.bar("Building package tree", &self.bar_template)
    }

    pub fn repo_loading(&self) -> ProgressBar {
        self.bar("Repository loading", &self.bar_template)
    }

    pub fn staging_loading(&self) -> ProgressBar {
        self.bar("Loading staging", &self.bar_template)
    }

    pub fn release_loading(&self) -> ProgressBar {
        self.bar("Loading releases", &self.bar_template)
    }

    pub fn what_depends(&self) -> ProgressBar {
        self.bar("Crawling dependencies", &self.bar_template)
    }

    pub fn job_bar(&self, id: &Uuid) -> ProgressBar {
        let b = self.bar(&format!("Job: {}", id), &self.bar_template);
        b.set_length(100);
        b
    }

    pub fn download_bar(&self, url: &Url) -> ProgressBar {
        self.bar(&format!("Download: {}", url.as_str()), &self.bar_template)
    }

    pub fn verification_bar(&self, path: PathBuf) -> ProgressBar {
        self.spinner(&self.spinner_template, format!("Verification: {}", path.display()))
    }

    fn bar(&self, msg: &str, template: &str) -> ProgressBar {
        if self.hide {
            ProgressBar::hidden()
        } else {
            let b = ProgressBar::new(1);
            b.set_style(ProgressStyle::default_bar().template(template));
            b.set_message(msg);
            b
        }
    }

    fn spinner(&self, template: &str, msg: String) -> ProgressBar {
        if self.hide {
            ProgressBar::hidden()
        } else {
            let bar = ProgressBar::new_spinner();
            bar.set_style(ProgressStyle::default_spinner().template(template));
            bar.enable_steady_tick(100);
            bar.set_message(&msg);
            bar
        }
    }

}

