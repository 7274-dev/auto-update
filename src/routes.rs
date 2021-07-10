use std::{ops::{DerefMut}, sync::{Arc}, time::Duration};

use futures::lock::Mutex;
use git2::Oid;
use rocket::{State};
use tokio::{io::{AsyncBufReadExt, BufReader}, time::timeout};

use crate::{Deployment, request::JsonResponse};

type DeploymentState<'a> = State<Arc<Mutex<Option<Deployment>>>>;

#[post("/deploy/<commit>?<password>")]
pub async fn deploy<'a>(commit: &str, deployment: &DeploymentState<'static>, correct_password: &State<String>, password: &str) -> JsonResponse<String> {
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
        Ok(dp) => {
            Some(dp)
        },
        Err(_) => return JsonResponse::new("Error!".to_string(), 400),
    };

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
            let mut reader = BufReader::new(stdout).lines();
            loop {
                let timeout = match timeout(Duration::from_millis(100), reader.next_line()).await {
                    Ok(t) => t,
                    Err(_) => break 
                };

                let line = match timeout {
                    Ok(t) => match t {
                        Some(t) => t,
                        None => break
                    },
                    Err(_) => break
                };
                
                deployment.logs += &line;
                deployment.logs += "\n";
            }

            deployment.logs.clone()
        },
        None => deployment.logs.clone()
    };

    JsonResponse::new(stdout, 200)
}

#[post("/stop?<password>")]
pub async fn stop_deployment(password: &str, correct_password: &State<String>, deployment: &DeploymentState<'static>) -> JsonResponse<String> {
    let password = password.to_string();
    let correct_password: String = correct_password.to_string();

    if password != correct_password {
        return JsonResponse::new("Incorrect password!".to_string(), 401);
    }

    let deployment = &mut deployment.lock().await;
    let ret;
    {
        ret = match &mut **deployment {
            Some(dep) => {
                match dep.process.kill().await {
                    Ok(_) => {
                        JsonResponse::new("Stopped current deployment!".to_string(), 200)
                    },
                    Err(_) => {
                        JsonResponse::new("Failed to stop current deployment!".to_string(), 500)
                    }
                }
            }
            None => JsonResponse::new("Nothing currently deployed!".to_string(), 400)
        }
        
    };

    **deployment = None;

    ret

}