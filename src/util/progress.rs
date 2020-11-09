use indicatif::*;
use uuid::Uuid;

pub struct ProgressBars {
    bar_template: String,
    hide: bool,
}

impl ProgressBars {
    pub fn setup(bar_template: String, hide: bool) -> Self {
        ProgressBars {
            bar_template,
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

    pub fn jobset_bar(&self, jobset_num: usize, number_of_jobsets: usize, jobs_in_jobset: usize) -> ProgressBar {
        let b = self.bar(&format!("Jobset {}/{} ({} Jobs)", jobset_num, number_of_jobsets, jobs_in_jobset), &self.bar_template);
        b.set_length(jobs_in_jobset as u64);
        b
    }

    pub fn job_bar(&self, id: &Uuid) -> ProgressBar {
        self.bar(&format!("Job: {}", id), &self.bar_template)
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

}


