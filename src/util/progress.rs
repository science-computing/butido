use indicatif::*;

pub struct ProgressBars {
    pub root:            MultiProgress,
    pub release_loading: ProgressBar,
    pub staging_loading: ProgressBar,
    pub repo_loading:    ProgressBar,
    pub tree_building:   ProgressBar,
}

impl ProgressBars {
    pub fn setup(max_packages: u64) -> Self {
        fn bar(msg: &str, max_packages: u64) -> ProgressBar {
            let b = ProgressBar::new(max_packages);
            b.set_style({
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {pos:>3}/{len:>3} ({percent:>3}%): {bar} | {msg}")
            });

            b.set_message(msg);
            b
        }

        let root = MultiProgress::new();
        ProgressBars {
            repo_loading:    root.add(bar("Repository loading", max_packages)),
            staging_loading: root.add(bar("Loading staging", max_packages)),
            release_loading: root.add(bar("Loading releases", max_packages)),
            tree_building:   root.add(bar("Building package tree", max_packages)),
            root,
        }
    }
}


