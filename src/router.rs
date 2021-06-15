use std::sync::{Arc, Mutex};

use git2::Oid;
use rocket::{Route, State};

use crate::{Deployment, request::JsonResponse};

type DeploymentState = State<Arc<Mutex<Deployment>>>;

#[post("/deploy/<commit>?<password>")]
pub fn deploy(commit: &str, deployment: &DeploymentState, correct_password: &State<String>, password: &str) -> JsonResponse<&'static str> {
    let correct_password = correct_password.to_string();
    if correct_password != password {
        return JsonResponse::new("Incorrect password!", 401);
    }

    let oid = match Oid::from_str(commit) {
        Ok(x) => x,
        Err(_) => return JsonResponse::new("Bad commit id.", 400)
    };

    
    let mut deployment = &mut *deployment.lock().unwrap();

    let new_deployment = match Deployment::deploy_commit(oid, Some(deployment)) {
        Ok(dp) => dp,
        Err(_) => return JsonResponse::new("Error!", 400)
    };

    deployment.process = new_deployment.process;
    deployment.commit_hash = new_deployment.commit_hash;

    JsonResponse::new("Successfully deployed commit!", 200)
}


pub fn routes() -> Vec<Route> {
    routes![deploy]
}