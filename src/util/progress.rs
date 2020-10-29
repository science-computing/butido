use indicatif::*;

pub struct ProgressBars(MultiProgress);

impl ProgressBars {
    pub fn setup() -> Self {
        ProgressBars(MultiProgress::new())
    }

    pub fn into_inner(self) -> MultiProgress {
        self.0
    }

    pub fn tree_building(&mut self) -> ProgressBar {
        self.0.add(Self::bar("Building package tree"))
    }

    pub fn repo_loading(&mut self) -> ProgressBar {
        self.0.add(Self::bar("Repository loading"))
    }

    pub fn staging_loading(&mut self) -> ProgressBar {
        self.0.add(Self::bar("Loading staging"))
    }

    pub fn release_loading(&mut self) -> ProgressBar {
        self.0.add(Self::bar("Loading releases"))
    }

    pub fn what_depends(&mut self) -> ProgressBar {
        self.0.add(Self::bar("Crawling dependencies"))
    }

    fn bar(msg: &str) -> ProgressBar {
        let b = ProgressBar::new(1);
        b.set_style({
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {pos:>3}/{len:>3} ({percent:>3}%): {bar} | {msg}")
        });

        b.set_message(msg);
        b
    }

}


