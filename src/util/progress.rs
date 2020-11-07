use indicatif::*;
use uuid::Uuid;

pub struct ProgressBars {
    bar_template: String,
}

impl ProgressBars {
    pub fn setup(bar_template: String) -> Self {
        ProgressBars {
            bar_template,
        }
    }

    pub fn tree_building(&self) -> ProgressBar {
        Self::bar("Building package tree", &self.bar_template)
    }

    pub fn repo_loading(&self) -> ProgressBar {
        Self::bar("Repository loading", &self.bar_template)
    }

    pub fn staging_loading(&self) -> ProgressBar {
        Self::bar("Loading staging", &self.bar_template)
    }

    pub fn release_loading(&self) -> ProgressBar {
        Self::bar("Loading releases", &self.bar_template)
    }

    pub fn what_depends(&self) -> ProgressBar {
        Self::bar("Crawling dependencies", &self.bar_template)
    }

    pub fn jobset_bar(&self, jobset_num: usize, number_of_jobsets: usize, jobs_in_jobset: usize) -> ProgressBar {
        let b = Self::bar(&format!("Jobset {}/{} ({} Jobs)", jobset_num, number_of_jobsets, jobs_in_jobset), &self.bar_template);
        b.set_length(jobs_in_jobset as u64);
        b
    }

    pub fn job_bar(&self, id: &Uuid) -> ProgressBar {
        Self::bar(&format!("Job: {}", id), &self.bar_template)
    }

    fn bar(msg: &str, template: &str) -> ProgressBar {
        let b = ProgressBar::new(1);
        b.set_style(ProgressStyle::default_bar().template(template));
        b.set_message(msg);
        b
    }

}


