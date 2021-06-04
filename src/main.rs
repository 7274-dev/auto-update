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
        let new_deployment_head = match repo.head() {
            Ok(head) => head,
            Err(_) => return Err(())
        };

        let new_deployment_tree = match new_deployment_head.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => return Err(()) 
        };

        let tree_len = new_deployment_tree.len();

        let current_deployment: &mut Deployment = match current_deployment.as_mut() {
            Some(mut_ref) => mut_ref,
            None => return Err(())
        };

        let deployed_commit = match repo.find_commit(current_deployment.commit_hash) {
            Ok(c) => c,
            Err(_) => return Err(())
        };

        let curr_deployment_tree = match deployed_commit.tree() {
            Ok(tree) => tree,
            Err(_) => return Err(())
        };

        let curr_deployment_tree_len = curr_deployment_tree.len();

        if tree_len <= curr_deployment_tree_len {
            return Err(());
        }

        current_deployment.process.kill().unwrap();
    }

    set_current_dir(Path::new(DEPLOYMENT_PATH)).unwrap();
    match Command::new(CHMOD_COMMAND.program).args(CHMOD_COMMAND.args.split(" ")).spawn() {
        Ok(mut x) => {
            match x.wait() {
                Ok(_) => (),
                Err(_) => return Err(())
            };
        },
        Err(_) => return Err(())
    }
    match Command::new(BUILD_COMMAND.program).args(BUILD_COMMAND.args.split(" ")).status() {
        Ok(status) => {
            if !status.success() {
                println!("Build failed.");
                return Err(())
            }
        },
        Err(_) => {
            return Err(())
        }
    };

    let mut target_jar_file: Option<PathBuf> = None; 

    match set_current_dir(Path::new("build/libs")) {
        Ok(_) => (),
        Err(_) => return Err(()) 
    };

    let files: ReadDir = match read_dir(Path::new(".")) {
        Ok(dir) => dir,
        Err(_) => return Err(())
    };

    for file in files {
        if file.is_err() {
            continue;
        }
        
        let file_ref = match file.as_ref() {
            Ok(r) => r,
            Err(_) => return Err(())
        };

        let file_path = file_ref.path(); 

        let extension = match file_path.extension() {
            Some(e) => e,
            None => return Err(())
        };

        if extension == "jar" {
            target_jar_file = Some(file.unwrap().path()); // safe to unwrap here as the file cannot be an error
        }
    }

    if target_jar_file == None {
        return Err(())
    }

    let target_jar_file = target_jar_file.unwrap(); // safe to unwrap here, we checked if the target_jar file is None
    let target_jar_file = target_jar_file.to_str().unwrap(); // safe to unwrap here too

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