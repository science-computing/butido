use anyhow::Result;
use log::{debug, trace};
use tokio::stream::StreamExt;

use crate::config::Configuration;
use crate::filestore::MergedStores;
use crate::job::Job;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::package::Shebang;
use crate::package::Tree;
use crate::package::PhaseName;
use crate::source::SourceCache;
use crate::util::docker::ImageName;

/// A set of jobs that could theoretically be run in parallel
#[derive(Debug)]
pub struct JobSet {
    set: Vec<Job>
}

impl JobSet {
    pub fn sets_from_tree(t: Tree, shebang: Shebang, image: ImageName, phases: Vec<PhaseName>, resources: Vec<JobResource>) -> Result<Vec<JobSet>> {
        tree_into_jobsets(t, shebang, image, phases, resources)
    }

    fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub async fn into_runables<'a>(self, merged_stores: &'a MergedStores, source_cache: &'a SourceCache, config: &Configuration) -> Result<Vec<RunnableJob>> {
        self.set
            .into_iter()
            .map(move |j| RunnableJob::build_from_job(j, merged_stores, source_cache, config))
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Result<Vec<RunnableJob>>>()
            .await
    }

}

/// Get the tree as sets of jobs, the deepest level of the tree first
fn tree_into_jobsets(tree: Tree, shebang: Shebang, image: ImageName, phases: Vec<PhaseName>, resources: Vec<JobResource>) -> Result<Vec<JobSet>> {
    fn inner(tree: Tree, shebang: &Shebang, image: &ImageName, phases: &Vec<PhaseName>, resources: &Vec<JobResource>) -> Result<Vec<JobSet>> {
        trace!("Creating jobsets for tree: {:?}", tree);

        let mut sets = vec![];
        let mut current_set = vec![];

        for (package, dep) in tree.into_iter() {
            trace!("Recursing for package: {:?}", package);
            let mut sub_sets = inner(dep, shebang, image, phases, resources)?; // recursion!
            sets.append(&mut sub_sets);
            current_set.push(package);
        }

        debug!("Jobset for set: {:?}", current_set);
        let jobset = JobSet {
            set: current_set
                .into_iter()
                .map(|package| {
                    Job::new(package, shebang.clone(), image.clone(), phases.clone(), resources.clone())
                })
                .collect(),
        };
        debug!("Jobset = {:?}", jobset);

        // make sure the current recursion is added _before_ all other recursions
        // which yields the highest level in the tree as _first_ element of the resulting vector
        let mut result = Vec::new();
        if !jobset.is_empty() {
            debug!("Adding jobset: {:?}", jobset);
            result.push(jobset)
        }
        result.append(&mut sets);
        debug!("Result =  {:?}", result);
        Ok(result)
    }

    inner(tree, &shebang, &image, &phases, &resources).map(|mut v| {
        // reverse, because the highest level in the tree is added as first element in the vector
        // and the deepest level is last.
        //
        // After reversing, we have a chain of things to build. Awesome, huh?
        v.reverse();
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::package::tests::package;
    use crate::package::Dependency;
    use crate::package::Dependencies;
    use crate::package::PhaseName;
    use crate::util::docker::ImageName;
    use crate::repository::Repository;

    use indicatif::ProgressBar;

    fn setup_logging() {
        let _ = ::env_logger::try_init();
    }

    #[test]
    fn test_one_element_tree_to_jobsets() {
        setup_logging();
        let mut btree = BTreeMap::new();

        let p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());

        let image  = ImageName::from(String::from("test"));
        let phases = vec![PhaseName::from(String::from("testphase"))];
        let shebang = Shebang::from(String::from("#!/bin/bash"));

        let js = JobSet::sets_from_tree(tree, shebang, image, phases, vec![]);
        assert!(js.is_ok());
        let js = js.unwrap();

        assert_eq!(js.len(), 1, "There should be only one jobset if there is only one element in the dependency tree: {:?}", js);

        let js = js.get(0).unwrap();
        assert_eq!(js.set.len(), 1, "The jobset should contain exactly one job: {:?}", js);

        let job = js.set.get(0).unwrap();
        assert_eq!(*job.package.name(), pname("a"), "The job should be for the package 'a': {:?}", job);
    }

    #[test]
    fn test_two_element_tree_to_jobsets() {
        setup_logging();
        let mut btree = BTreeMap::new();

        let p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let p2 = {
            let name = "b";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());

        let r = tree.add_package(p2, &repo, progress.clone());
        assert!(r.is_ok());

        let image  = ImageName::from(String::from("test"));
        let phases = vec![PhaseName::from(String::from("testphase"))];
        let shebang = Shebang::from(String::from("#!/bin/bash"));

        let js = JobSet::sets_from_tree(tree, shebang, image, phases, vec![]);
        assert!(js.is_ok());
        let js = js.unwrap();

        assert_eq!(js.len(), 1, "There should be one set of jobs for two packages on the same level in the tree: {:?}", js);

        let js = js.get(0).unwrap();
        assert_eq!(js.set.len(), 2, "The jobset should contain exactly two jobs: {:?}", js);

        let job = js.set.get(0).unwrap();
        assert_eq!(*job.package.name(), pname("a"), "The job should be for the package 'a': {:?}", job);

        let job = js.set.get(1).unwrap();
        assert_eq!(*job.package.name(), pname("b"), "The job should be for the package 'a': {:?}", job);
    }

    #[test]
    fn test_two_dependent_elements_to_jobsets() {
        setup_logging();
        let mut btree = BTreeMap::new();

        let p1 = {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("b =2"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let _ = {
            let name = "b";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());

        let image  = ImageName::from(String::from("test"));
        let phases = vec![PhaseName::from(String::from("testphase"))];
        let shebang = Shebang::from(String::from("#!/bin/bash"));

        let js = JobSet::sets_from_tree(tree, shebang, image, phases, vec![]);
        assert!(js.is_ok());
        let js = js.unwrap();

        assert_eq!(js.len(), 2, "There should be two set of jobs for two packages where one depends on the other: {:?}", js);

        {
            let first_js = js.get(0).unwrap();
            assert_eq!(first_js.set.len(), 1, "The first jobset should contain exactly one job: {:?}", js);

            let job = first_js.set.get(0).unwrap();
            assert_eq!(*job.package.name(), pname("b"), "The job from the first set should be for the package 'b': {:?}", job);
        }

        {
            let second_js = js.get(1).unwrap();
            assert_eq!(second_js.set.len(), 1, "The second jobset should contain exactly one job: {:?}", js);

            let job = second_js.set.get(0).unwrap();
            assert_eq!(*job.package.name(), pname("a"), "The job should be for the package 'a': {:?}", job);
        }

    }

}

