use std::sync::{Arc, RwLock};

use rocket::http::Status;
use rocket::response::status;
use rocket::response::status::Custom;
use rocket::response::NamedFile;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};

pub type Session<'a> = rocket_session::Session<'a, bool>;

use crate::{write_config_logged, BotConfig, Tag};

fn logged_in(session: &Session) -> bool {
    session.tap(|b| *b)
}

#[get("/?<key>")]
fn index(session: Session, key: Option<String>) -> Result<NamedFile, status::Custom<String>> {
    NamedFile::open("index.html").map_err(|e| Custom(Status::NotFound, e.to_string()))
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Login {
    key: String,
}

#[post("/login", data = "<key>", format = "json")]
fn login(
    session: Session,
    key: Json<Login>,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
) -> Status {
    if logged_in(&session) {
        Status::Ok
    } else {
        let t = bc.write().unwrap();
        if key.into_inner().key == t.key {
            session.tap(|b| *b = true);
            Status::Ok
        } else {
            Status::Forbidden
        }
    }
}

#[post("/add", data = "<tag>", format = "json")]
fn add_tag(
    session: Session,
    tag: Json<Tag>,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Status {
    let mut t = bc.write().unwrap();
    if logged_in(&session) {
        t.tags.push(tag.into_inner());
        write_config_logged(&config_file, &t);
        Status::Created
    } else {
        Status::Forbidden
    }
}

#[get("/")]
fn get_tags(
    session: Session,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
) -> Result<Json<Vec<crate::Tag>>, Status> {
    let t = bc.read().unwrap();
    if logged_in(&session) {
        Ok(Json(t.tags.clone()))
    } else {
        Err(Status::Forbidden)
    }
}

#[delete("/<id>")]
fn delete_tag(
    session: Session,
    id: usize,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Status {
    let mut t = bc.write().unwrap();
    if logged_in(&session) && id < t.tags.len() {
        let tag = t.tags.remove(id);
        write_config_logged(&config_file, &t);
        Status::Ok
    } else {
        Status::Forbidden
    }
}

#[put("/<id>", data = "<tag>", format = "json")]
fn update_tag(
    session: Session,
    id: usize,
    tag: Json<Tag>,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Status {
    let mut t = bc.write().unwrap();
    if logged_in(&session) && id < t.tags.len() {
        t.tags[id] = tag.into_inner();
        write_config_logged(&config_file, &t);
        Status::Ok
    } else {
        Status::Forbidden
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Message {
    name: String,
    value: String,
}

#[get("/<name>")]
fn get_message(
    session: Session,
    name: String,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Result<Json<Message>, Status> {
    if logged_in(&session) {
        let t = bc.read().unwrap();
        match name.as_str() {
            "response_message_success" => Ok(Json(Message {
                name,
                value: t.response_message_success.clone(),
            })),
            "response_message_failure" => Ok(Json(Message {
                name,
                value: t.response_message_failure.clone(),
            })),
            _ => Err(Status::NotFound),
        }
    } else {
        return Err(Status::Forbidden);
    }
}

#[post("/<name>", data = "<msg>", format = "json")]
fn set_message(
    session: Session,
    name: String,
    msg: Json<String>,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Status {
    if logged_in(&session) {
        let mut t = bc.write().unwrap();
        let msg = msg.into_inner();
        match name.as_str() {
            "response_message_success" => {
                t.response_message_success = msg;
                Status::Created
            }
            "response_message_failure" => {
                t.response_message_failure = msg;
                Status::Created
            }
            _ => Status::NotFound,
        }
    } else {
        return Status::Forbidden;
    }
}

#[get("/")]
fn get_messages(
    session: Session,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> Result<Json<Vec<Message>>, Status> {
    if logged_in(&session) {
        let t = bc.read().unwrap();
        Ok(Json(vec![
            Message {
                name: "response_message_success".to_string(),
                value: t.response_message_success.clone(),
            },
            Message {
                name: "response_message_failure".to_string(),
                value: t.response_message_failure.clone(),
            },
        ]))
    } else {
        return Err(Status::Forbidden);
    }
}

pub fn rocket(bc: Arc<RwLock<BotConfig>>, config_file: String) -> rocket::Rocket {
    rocket::ignite()
        .manage(bc)
        .manage(config_file)
        .mount("/", routes![index, login])
        .mount("/tags", routes![add_tag, delete_tag, get_tags, update_tag])
        .mount("/messages", routes![get_message, get_messages, set_message])
        .attach(Session::fairing())
}

pub fn rocket_main(bc: Arc<RwLock<BotConfig>>, config_file: String) {
    rocket(bc, config_file).launch();
}
