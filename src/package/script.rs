use anyhow::Error;
use anyhow::Result;
use handlebars::Handlebars;
use serde::Deserialize;
use serde::Serialize;

use crate::package::Package;
use crate::phase::Phase;
use crate::phase::PhaseName;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Script(String);

impl AsRef<str> for Script {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub struct ScriptBuilder<'a> {
    shebang : &'a String,
}

impl<'a> ScriptBuilder<'a> {
    // TODO: Use handlebars and templating instead of hardcoding
    pub fn new(shebang: &'a String) -> Self {
        ScriptBuilder {
            shebang,
        }
    }

    pub fn build(self, package: &Package, phaseorder: &Vec<PhaseName>) -> Result<Script> {
        let mut script = format!("{shebang}\n", shebang = self.shebang);

        for name in phaseorder {
            match package.phases().get(name) {
                Some(Phase::Text(text)) => {
                    script.push_str(&indoc::formatdoc!(r#"
                        ### phase {}
                        {}
                        ### / {} phase
                    "#,
                    name.as_str(),
                    text,
                    name.as_str(),
                    ));

                    script.push_str("\n");
                },

                // TODO: Support path embedding
                // (requires possibility to have stuff in Script type that gets copied to
                // container)
                Some(Phase::Path(pb)) => {
                    script.push_str(&format!(r#"
                        # Phase (from file {path}): {name}
                        # NOT SUPPORTED YET
                        exit 1
                    "#,
                    path = pb.display(),
                    name = name.as_str()));

                    script.push_str("\n");
                },

                None => {
                    script.push_str(&format!("# No script for phase: {name}", name = name.as_str()));
                    script.push_str("\n");
                },
            }
        }

        Self::interpolate_package(script, package).map(Script)
    }

    fn interpolate_package(script: String, package: &Package) -> Result<String> {
        let mut hb = Handlebars::new();
        hb.register_template_string("script", script)?;
        hb.render("releases", package).map_err(Error::from)
    }
}
