use anyhow::Result;

use crate::phase::Phase;
use crate::phase::PhaseName;
use crate::package::Package;

#[derive(Debug)]
pub struct Script(String);

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
                    script.push_str(&format!(r#"
                        ### phase {name}
                        {text}
                        ### / phase
                    "#,
                    name = name.as_str(),
                    text = text,
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

        Ok(Script(script))
    }
}
