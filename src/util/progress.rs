use indicatif::*;

pub struct ProgressBars {
    multi: MultiProgress,
    bar_template: String,
}

impl ProgressBars {
    pub fn setup(bar_template: String) -> Self {
        ProgressBars {
            multi: MultiProgress::new(),
            bar_template,
        }
    }

    pub fn into_inner(self) -> MultiProgress {
        self.multi
    }

    pub fn tree_building(&mut self) -> ProgressBar {
        self.multi.add(Self::bar("Building package tree", &self.bar_template))
    }

    pub fn repo_loading(&mut self) -> ProgressBar {
        self.multi.add(Self::bar("Repository loading", &self.bar_template))
    }

    pub fn staging_loading(&mut self) -> ProgressBar {
        self.multi.add(Self::bar("Loading staging", &self.bar_template))
    }

    pub fn release_loading(&mut self) -> ProgressBar {
        self.multi.add(Self::bar("Loading releases", &self.bar_template))
    }

    pub fn what_depends(&mut self) -> ProgressBar {
        self.multi.add(Self::bar("Crawling dependencies", &self.bar_template))
    }

    fn bar(msg: &str, template: &str) -> ProgressBar {
        let b = ProgressBar::new(1);
        b.set_style(ProgressStyle::default_bar().template(template));
        b.set_message(msg);
        b
    }

}


