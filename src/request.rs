use std::io::Cursor;

use rocket::{http::{ContentType, Status}, response::Responder};
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct JsonResponse<T> {
    pub response: T,
    pub status_code: u16
}


impl<'r, T> Responder<'r, 'r> for JsonResponse<T> where T: Serialize {
    fn respond_to(self, _request: &rocket::Request) -> rocket::response::Result<'r> {
        let json_string = match serde_json::to_string_pretty(&self) {
            Ok(result) => result,
            Err(_) => return Result::Err(Status::InternalServerError)
        };

        let response = rocket::Response::build()
            .sized_body(json_string.len(), Cursor::new(json_string))
            .header(ContentType::new("application", "json"))
            .status(Status::from_code(self.status_code).unwrap())
            .finalize();


        Result::Ok(response)
    }
}

impl<T> JsonResponse<T> {
    pub fn new(response: T, status_code: u16) -> Self {
        JsonResponse { response, status_code }
    }
}
