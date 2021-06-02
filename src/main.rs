extern crate git2;

use std::{env::set_current_dir, fs::{ReadDir, create_dir, read_dir, remove_dir_all}, path::{Path, PathBuf}, process::{Child, Command}};

use git2::{Oid, Repository};

struct Cmd {
    pub program: &'static str,
    pub args: &'static str
}

const REPOSITORY_URL: &str = "https://github.com/7274-dev/AdventnaVyzva-GlobalBackend.git";
const DEPLOYMENT_PATH: &str  = "deployment"; // has to be absolute

const CHMOD_COMMAND: Cmd = Cmd { program: "chmod", args: "+x gradlew" };
const BUILD_COMMAND: Cmd = Cmd { program: "./gradlew", args: "bootJar" }; 
const DEPLOY_COMMAND: Cmd = Cmd { program: "java", args: "-jar %s" };

struct Deployment {
    pub commit_hash: Oid,
    pub process: Child
}

fn deploy_commit(commit: Oid, mut current_deployment: Option<Deployment>) -> Result<Deployment, ()> {
    if Path::new(DEPLOYMENT_PATH).exists() {
        remove_dir_all(Path::new(DEPLOYMENT_PATH)).unwrap();
    }

    create_dir(Path::new(DEPLOYMENT_PATH)).unwrap();

    let repo = match Repository::clone(REPOSITORY_URL, DEPLOYMENT_PATH) {
        Ok(repo) => repo,
        Err(_) => return Err(())
    };

    match repo.set_head_detached(commit) {
        Ok(_) => (),
        Err(_) => return  Err(()) 
    };

    if current_deployment.as_ref().is_some() {
        let new_deployment_head = repo.head().unwrap();
        let new_deployment_tree = new_deployment_head.peel_to_tree().unwrap();

        let tree_len = new_deployment_tree.len();

        let current_deployment: &mut Deployment = current_deployment.as_mut().unwrap();
        let deployed_commit = repo.find_commit(current_deployment.commit_hash).unwrap();
        let curr_deployment_tree_len = deployed_commit.tree().unwrap().len();

        if tree_len < curr_deployment_tree_len {
            return Err(());
        }

        current_deployment.process.kill().unwrap();
    }

    set_current_dir(Path::new(DEPLOYMENT_PATH)).unwrap();
    Command::new(CHMOD_COMMAND.program)
                    .args(CHMOD_COMMAND.args.split(" "))
                    .spawn().unwrap()
                    .wait().unwrap();
    match Command::new(BUILD_COMMAND.program)
                    .args(BUILD_COMMAND.args.split(" ")).status() {
        Ok(status) => {
            if !status.success() {
                println!("Build failed.");
                return Err(())
            }
        },
        Err(e) => {
            println!("{:?}", e);
            return Err(())
        }
    };

    let mut target_jar_file: Option<PathBuf> = None; 

    set_current_dir(Path::new("build/libs")).unwrap();
    let files: ReadDir = read_dir(Path::new(".")).unwrap();
    for file in files {
        if file.is_err() {
            continue;
        }

        if file.as_ref().unwrap().path().extension().unwrap() == "jar" {
            target_jar_file = Some(file.unwrap().path());
        }
    }

    if target_jar_file == None {
        return Err(())
    }
    let target_jar_file = target_jar_file.unwrap();
    let target_jar_file = target_jar_file.to_str().unwrap();

    let args_with_jarfile = DEPLOY_COMMAND.args.replace("%s", target_jar_file);

    let child = match Command::new(DEPLOY_COMMAND.program)
            .args(args_with_jarfile.split(" "))
            .spawn() {
        Ok(child) => child,
        Err(_) => {
            println!("Deployment failed.");
            return Err(())
        }
    };

    let new_deployment = Deployment { commit_hash: commit, process: child };

    Ok(new_deployment)
}

fn main() {
    let commit = Oid::from_str("a4632144411f10ec52dc94e9fcc5fc91fa11bd19").unwrap();

    match deploy_commit(commit, None) {
        Ok(d) => {
            println!("Yay!");
        },
        Err(_) => {
            println!("ffs");
        }
    };


}