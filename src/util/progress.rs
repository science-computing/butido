//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use indicatif::*;

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

    pub fn bar(&self) -> ProgressBar {
        if self.hide {
            ProgressBar::hidden()
        } else {
            let b = ProgressBar::new(1);
            b.set_style(ProgressStyle::default_bar().template(&self.bar_template));
            b
        }
    }

    pub fn spinner(&self) -> ProgressBar {
        if self.hide {
            ProgressBar::hidden()
        } else {
            let bar = ProgressBar::new_spinner();
            bar.set_style(ProgressStyle::default_spinner().template(&self.spinner_template));
            bar
        }
    }
}
