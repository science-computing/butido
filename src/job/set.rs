use anyhow::Result;

use crate::job::Job;
use crate::package::Package;
use crate::package::Tree;
use crate::phase::PhaseName;
use crate::util::docker::ImageName;

/// A set of jobs that could theoretically be run in parallel
#[derive(Debug)]
pub struct JobSet {
    set: Vec<Job>
}

impl JobSet {
    pub fn sets_from_tree(t: Tree, image: ImageName, phases: Vec<PhaseName>) -> Result<Vec<JobSet>> {
        tree_into_jobsets(t, image, phases)
    }

}

/// Get the tree as sets of jobs, the deepest level of the tree first
fn tree_into_jobsets(tree: Tree, image: ImageName, phases: Vec<PhaseName>) -> Result<Vec<JobSet>> {
    fn inner(tree: Tree, image: &ImageName, phases: &Vec<PhaseName>) -> Result<Vec<JobSet>> {
        let mut sets = vec![];
        let mut current_set = vec![];

        for (package, dep) in tree.into_iter() {
            let mut sub_sets = inner(dep, image, phases)?; // recursion!
            sets.append(&mut sub_sets);
            current_set.push(package);
        }

        let jobset = JobSet {
            set: current_set
                .into_iter()
                .map(|package| {
                    Job::new(package, image.clone(), phases.clone())
                })
                .collect(),
        };

        // make sure the current recursion is added _before_ all other recursions
        // which yields the highest level in the tree as _first_ element of the resulting vector
        let mut result = vec![jobset];
        result.append(&mut sets);
        Ok(result)
    }

    inner(tree, &image, &phases).map(|mut v| {
        // reverse, because the highest level in the tree is added as first element in the vector
        // and the deepest level is last.
        //
        // After reversing, we have a chain of things to build. Awesome, huh?
        v.reverse();
        v
    })
}

