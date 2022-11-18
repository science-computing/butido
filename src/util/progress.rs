//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use indicatif::*;
use getset::CopyGetters;

#[derive(Clone, Debug, CopyGetters)]
pub struct ProgressBars {
    bar_template: String,

    #[getset(get_copy = "pub")]
    hide: bool,
}

impl ProgressBars {
    pub fn setup(bar_template: String, hide: bool) -> Self {
        ProgressBars {
            bar_template,
            hide,
        }
    }

    pub fn bar(&self) -> anyhow::Result<ProgressBar> {
        if self.hide {
            Ok(ProgressBar::hidden())
        } else {
            let b = ProgressBar::new(1);
            b.set_style(ProgressStyle::default_bar().template(&self.bar_template)?);
            Ok(b)
        }
    }
}
