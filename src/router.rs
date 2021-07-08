use rocket::Route;

use crate::routes::*;

pub fn routes() -> Vec<Route> {
    routes![deploy, get_logs, stop_deployment]
}