extern crate git2;

use std::process::{Child, Command};

use git2::{Oid, Repository};

const REPOSITORY_URL: &'static str = "https://github.com/7274-dev/AdventnaVyzva-GlobalBackend.git";
const DEPLOYMENT_PATH: &'static str = "deployment";

struct Deployment {
    pub commit_hash: Oid,
    pub process: Child
}

fn deploy_commit(commit: Oid, current_deployment: Option<Deployment>) -> Result<Deployment, ()> {
    // remove previous downloaded repo

    let repo = match Repository::clone(REPOSITORY_URL, DEPLOYMENT_PATH) {
        Ok(repo) => repo,
        Err(e) => return Err(())
    };

    match repo.set_head_detached(commit) {
        Ok(_) => (),
        Err(_) => return  Err(()) 
    };

    if current_deployment.is_some() {
        let new_deployment_head = repo.head().unwrap();
        let new_deployment_tree = new_deployment_head.peel_to_tree().unwrap();

        let tree_len = new_deployment_tree.len();

        let current_deployment = current_deployment.unwrap();
        let deployed_commit = repo.find_commit(current_deployment.commit_hash).unwrap();
        let curr_deployment_tree_len = deployed_commit.tree().unwrap().len();
        
        if tree_len < curr_deployment_tree_len {
            return Err(());
        }
        
        return Err(())
    }

    Err(())

}

fn main() {

}