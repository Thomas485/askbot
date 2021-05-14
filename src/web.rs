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
fn index(_session: Session, key: Option<String>) -> Result<NamedFile, status::Custom<String>> {
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
        let _ = t.tags.remove(id);
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
    _config_file: rocket::State<String>,
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
    _config_file: rocket::State<String>,
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
    _config_file: rocket::State<String>,
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

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::http::Status;
    use rocket::local::Client;
    use std::sync::{Arc, RwLock};

    fn prepare_client() -> rocket::local::Client {
        match crate::read_config("testconfig.json") {
            Ok(bc) => {
                let test_bc = Arc::new(RwLock::new(bc));
                return Client::new(rocket(test_bc, "testconfig.json".to_string()))
                    .expect("valid rocket instance");
            }
            Err(e) => panic!("{}", e),
        }
    }
    fn prepare_client_bc() -> (rocket::local::Client, Arc<RwLock<crate::BotConfig>>) {
        match crate::read_config("testconfig.json") {
            Ok(bc) => {
                let test_bc = Arc::new(RwLock::new(bc));
                return (
                    Client::new(rocket(Arc::clone(&test_bc), "testconfig.json".to_string()))
                        .expect("valid rocket instance"),
                    test_bc,
                );
            }
            Err(e) => panic!("{}", e),
        }
    }

    fn do_login(client: &mut rocket::local::Client) {
        client
            .post("/login")
            .header(rocket::http::ContentType::JSON)
            .body("{\"key\": \"foo\"}")
            .dispatch();
    }

    #[test]
    fn login() {
        let client = prepare_client();

        // not logged in.
        let response = client.post("/login").dispatch();
        // NotFound because the key is mandatory.
        // Should be fine.
        assert_eq!(response.status(), Status::NotFound);

        // wrong login
        let response = client
            .post("/login")
            .header(rocket::http::ContentType::JSON)
            .body("{\"key\": \"wrong\"}")
            .dispatch();
        assert_eq!(response.status(), Status::Forbidden);

        // login
        let mut response = client
            .post("/login")
            .header(rocket::http::ContentType::JSON)
            .body("{\"key\": \"foo\"}")
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), None);
    }

    #[test]
    fn get_tags() {
        let (mut client, bc) = prepare_client_bc();

        do_login(&mut client);

        let mut response = client.get("/tags/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        // response data == server data
        assert_eq!(
            response.body_string().map(|mut s| s
                .drain(..)
                .filter(|c| *c != '\n' && *c != ' ')
                .collect::<String>()),
            serde_json::to_string(&bc.read().unwrap().tags).ok()
        );
    }

    #[test]
    fn add_tag() {
        let (mut client, bc) = prepare_client_bc();

        do_login(&mut client);

        let old_count = bc.read().unwrap().tags.len();
        let mut response = client
            .post("/tags/add")
            .header(rocket::http::ContentType::JSON)
            .body(
                rocket_contrib::json!({
                    "tag": format!("#test{}",old_count+1),
                    "webhook": format!("test{}-hook",old_count+1)
                })
                .to_string(),
            )
            .dispatch();
        assert_eq!(response.status(), Status::Created);
        assert_eq!(response.body_string(), None);
        let new_count = bc.read().unwrap().tags.len();
        assert_eq!(new_count, old_count + 1);
    }

    #[test]
    fn delete_tag() {
        let (mut client, bc) = prepare_client_bc();
        do_login(&mut client);

        // get old data
        let old_count = bc.read().unwrap().tags.len();
        assert!(old_count > 0);

        // delete
        let response = client.delete(format!("/tags/{}", old_count - 1)).dispatch();
        assert_eq!(response.status(), Status::Ok);

        // check
        let new_count = bc.read().unwrap().tags.len();
        assert_eq!(new_count, old_count - 1);
    }

    #[test]
    fn update_tag() {
        let (mut client, bc) = prepare_client_bc();
        do_login(&mut client);

        // get old data
        let old_count = bc.read().unwrap().tags.len();
        assert!(old_count > 0);
        let mut old_tag = bc.read().unwrap().tags[0].clone();

        let mut new_tag = old_tag.clone();
        let number = old_tag.tag.split_off(5).parse::<i32>().unwrap();
        new_tag.tag = format!("#test{}", (number + 1) % 100);

        // update
        let response = client
            .put(format!("/tags/{}", 0))
            .header(rocket::http::ContentType::JSON)
            .body(
                rocket_contrib::json!({
                    "tag": new_tag.tag,
                    "webhook": new_tag.webhook
                })
                .to_string(),
            )
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        // check
        let new_count = bc.read().unwrap().tags.len();
        assert_eq!(new_count, old_count);
        let updated_tag = bc.read().unwrap().tags[0].clone();
        assert_eq!(new_tag, updated_tag);
    }
}
