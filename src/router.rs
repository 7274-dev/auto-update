use std::{borrow::Cow, io::{BufRead, BufReader}, ops::Deref, sync::{Arc, Mutex}};

use git2::Oid;
use rocket::{Route, State, response::content::Json};
use serde::{Serialize, Deserialize};

use crate::{Deployment, request::JsonResponse};

type DeploymentState<'a> = State<Arc<Mutex<Option<&'a mut Deployment>>>>;

#[derive(Serialize, Deserialize)]
struct ProgramOutput {
    pub stdout: String,
    pub stderr: String
}

#[post("/deploy/<commit>?<password>")]
pub fn deploy(commit: &str, deployment: &DeploymentState, correct_password: &State<String>, password: &str) -> JsonResponse<String> {
    let correct_password = correct_password.to_string();
    if correct_password != password {
        return JsonResponse::new("Incorrect password!".to_string(), 401);
    }

    let oid = match Oid::from_str(commit) {
        Ok(x) => x,
        Err(_) => return JsonResponse::new("Bad commit id.".to_string(), 400)
    };

    let mut deployment =  deployment.lock().unwrap();
    let mut deployment = deployment.as_deref_mut();

    let mut new_deployment = match Deployment::deploy_commit(oid, deployment.as_deref_mut()) {
        Ok(dp) => dp,
        Err(_) => return JsonResponse::new("Error!".to_string(), 400)
    };

    if deployment.is_none() {
        deployment = Some(&mut new_deployment);
    }
    else {
        let mut deployment = deployment.unwrap();
        deployment.process = new_deployment.process;
        deployment.commit_hash = new_deployment.commit_hash;
    }

    JsonResponse::new("Successfully deployed commit!".to_string(), 200)
}

#[get("/logs?<password>")]
pub fn get_logs(password: &str, correct_password: &State<String>, deployment: &DeploymentState) -> JsonResponse<String> {
    let password = password.to_string();
    let correct_password: String = correct_password.to_string();

    if password != correct_password {
        return JsonResponse::new("Incorrect password!".to_string(), 401);
    }

    let deployment = &mut *deployment.lock().unwrap();
    if deployment.is_none() {
        return JsonResponse::new("Nothing currently deployed!".to_string(), 400);
    }

    let deployment = deployment.as_deref_mut().unwrap();



    let stdout = match &mut deployment.process.stdout {
        Some(stdout) => {
            let reader = BufReader::new(stdout);
            let mut output = String::new();
            reader.lines()
                    .filter_map(|line| line.ok())
                    .for_each(|line| output += &line);
            output
        },
        None => "".to_string()
    };

    let stderr = match &mut deployment.process.stderr {
        Some(stderr) => {
            let reader = BufReader::new(stderr);
            let mut output = String::new();
            reader.lines()
                    .filter_map(|line| line.ok())
                    .for_each(|line| output += &line);
            output
        },
        None => "".to_string()
    };

    let program_output = ProgramOutput { stdout, stderr };
    let program_output = match serde_json::to_string(&program_output) {
        Ok(s) => s,
        Err(_) => return JsonResponse::new("Failed to serialize program output".to_string(), 500)
    };

    JsonResponse::new(program_output.clone(), 200)
}


pub fn routes() -> Vec<Route> {
    routes![deploy]
}