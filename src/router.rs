use std::{ops::{DerefMut}, sync::{Arc}, time::Duration};

use futures::lock::Mutex;
use git2::Oid;
use rocket::{Route, State};
use serde::{Serialize, Deserialize};
use tokio::{io::{AsyncBufReadExt, BufReader}, time::timeout};

use crate::{Deployment, request::JsonResponse};

type DeploymentState<'a> = State<Arc<Mutex<Option<Deployment>>>>;

#[derive(Serialize, Deserialize)]
struct ProgramOutput {
    pub stdout: String,
    pub stderr: String
}

#[post("/deploy/<commit>?<password>")]
async fn deploy<'a>(commit: &str, deployment: &DeploymentState<'static>, correct_password: &State<String>, password: &str) -> JsonResponse<String> {
    let correct_password = correct_password.to_string();
    if correct_password != password {
        return JsonResponse::new("Incorrect password!".to_string(), 401);
    }

    let oid = match Oid::from_str(commit) {
        Ok(x) => x,
        Err(_) => return JsonResponse::new("Bad commit id.".to_string(), 400)
    };

    let mut deployment = deployment.lock().await;

    *deployment = match Deployment::deploy_commit(oid, deployment.deref_mut()).await {
        Ok(dp) => Some(dp),
        Err(_) => return JsonResponse::new("Error!".to_string(), 400),
    };

    // *deployment = Some(&mut new_deployment.into_inner());

    // if deployment.is_none() {
    //       *deployment = Some(&mut new_deployment);
    // }
    // else {
    //     let mut deployment = deployment.unwrap();
    //     deployment.process = new_deployment.process;
    //     deployment.commit_hash = new_deployment.commit_hash;
    // }

    JsonResponse::new("Successfully deployed commit!".to_string(), 200)
}

#[get("/logs?<password>")]
pub async fn get_logs(password: &str, correct_password: &State<String>, deployment: &DeploymentState<'static>) -> JsonResponse<String> {
    let password = password.to_string();
    let correct_password: String = correct_password.to_string();

    if password != correct_password {
        return JsonResponse::new("Incorrect password!".to_string(), 401);
    }

    let deployment = &mut *deployment.lock().await;
    let deployment = &mut match deployment {
        Some(dep) => dep,
        None => return JsonResponse::new("Nothing currently deployed!".to_string(), 400)
    };

    let stdout = match &mut deployment.process.stdout {
        Some(stdout) => {
            let mut output = String::new();
            let mut reader = BufReader::new(stdout).lines();
            loop {
                let timeout = match timeout(Duration::from_millis(100), reader.next_line()).await {
                    Ok(t) => t,
                    Err(_) => break 
                };

                let line = match timeout {
                    Ok(t) => t.unwrap(),
                    Err(_) => break
                };
                
                output += &line;
            }

            output
        },
        None => "".to_string()
    };

    // let stderr = match &mut deployment.process.stderr {
    //     Some(stderr) => {
    //         let reader = BufReader::new(stderr);
    //         let mut output = String::new();
    //         reader.lines()
    //                 .filter_map(|line| line.ok())
    //                 .for_each(|line| output += &line);
    //         output
    //     },
    //     None => "".to_string()
    // };

    // let program_output = ProgramOutput { stdout, stderr };
    // let program_output = match serde_json::to_string(&program_output) {
        // Ok(s) => s,
        // Err(_) => return JsonResponse::new("Failed to serialize program output".to_string(), 500)
    // };

    // JsonResponse::new(program_output.clone(), 200)
    JsonResponse::new(stdout, 200)
}


pub fn routes() -> Vec<Route> {
    routes![deploy, get_logs]
}