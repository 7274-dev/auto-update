extern crate git2;
#[macro_use] extern crate rocket;

use std::{env::set_current_dir, fs::{ReadDir, create_dir, read_dir, remove_dir_all}, path::{Path, PathBuf}, sync::{Arc}};

use async_process::Stdio;
use dotenv::dotenv;

use futures::lock::Mutex;
use git2::{Oid, Repository};

use tokio::{process::{Child, Command}};

use crate::router::routes;

pub mod router;
pub mod request;
pub mod routes;

struct Cmd {
    pub program: &'static str,
    pub args: &'static str
}

const REPOSITORY_URL: &str = "https://github.com/7274-dev/AdventnaVyzva-GlobalBackend.git";
const DEPLOYMENT_PATH: &str  = "deployment"; // has to be absolute

const CHMOD_COMMAND: Cmd = Cmd { program: "chmod", args: "+x gradlew" };
const BUILD_COMMAND: Cmd = Cmd { program: "./gradlew", args: "bootJar" }; 
const DEPLOY_COMMAND: Cmd = Cmd { program: "java", args: "-jar %s" };

pub struct Deployment {
    pub commit_hash: Oid,
    pub process: Child,
    pub logs: String
}

impl Deployment {
    async fn deploy_commit<'a>(commit: Oid, current_deployment: &mut Option<Deployment>) -> Result<Deployment, ()> {
        if Path::new(DEPLOYMENT_PATH).exists() {
            match remove_dir_all(Path::new(DEPLOYMENT_PATH)) {
                Ok(_) => (),
                Err(_) => return Err(())
            };
        }

    
        match create_dir(Path::new(DEPLOYMENT_PATH)) {
            Ok(_) => (),
            Err(_) => return Err(())
        };
    
        let repo = match Repository::clone(REPOSITORY_URL, DEPLOYMENT_PATH) {
            Ok(repo) => repo,
            Err(_) => return Err(())
        };
    
        match repo.set_head_detached(commit) {
            Ok(_) => (),
            Err(_) => return  Err(()) 
        };
    
        if current_deployment.is_some() {
            // a lot of chained matches to avoid lifetime errors here
            let current_deployment: &mut Deployment = match current_deployment {
                Some(mut_ref) => mut_ref,
                None => return Err(())
            };
    
            let curr_deployment_tree_len = match repo.find_commit(current_deployment.commit_hash) {
                Ok(c) => match c.tree() {
                    Ok(tree) => tree.len(),
                    Err(_) => return Err(())
                },
                Err(_) => return Err(())
            };
    
            if match repo.head() {
                Ok(head) => match head.peel_to_tree() {
                    Ok(tree) => tree.len(),
                    Err(_) => return Err(()) 
                },
                Err(_) => return Err(())
            } <= curr_deployment_tree_len {
                return Err(());
            }
    
            match current_deployment.process.kill().await {
                Ok(_) => (),
                Err(_) => return Err(())
            };
        }
    
        set_current_dir(Path::new(DEPLOYMENT_PATH)).unwrap();
        match &mut Command::new(CHMOD_COMMAND.program).args(CHMOD_COMMAND.args.split(" ")).spawn() {
            Ok(c) => {
                match c.wait().await {
                    Ok(_) => (),
                    Err(_) => return Err(())
                }
            },
            Err(_) => return Err(())
        };

        match Command::new(BUILD_COMMAND.program).args(BUILD_COMMAND.args.split(" ")).status().await {
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
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn() {
            Ok(child) => child,
            Err(_) => {
                println!("Deployment failed.");
                return Err(())
            }
        };
    
        let new_deployment = Deployment { commit_hash: commit, process: child, logs: String::new() };
    
        Ok(new_deployment)
    }
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let password = match std::env::var("PASSWORD") {
        Ok(v) => v,
        Err(_) => panic!("The password enviroment variable is not set!")
    };

    let deployment: Arc<Mutex<Option<Deployment>>> = Arc::new(Mutex::new(None));

    rocket::build()
        .manage(password) // password state
        .manage(deployment) // deployment state
        .mount("/", routes())
}