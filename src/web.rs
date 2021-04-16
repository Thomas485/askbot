use std::sync::{Arc, RwLock};

use rocket::http::Status;
use rocket::response::status;
use rocket::response::status::Custom;
use rocket::response::NamedFile;
use rocket_contrib::json::Json;

pub type Session<'a> = rocket_session::Session<'a, bool>;

use crate::{write_config_logged, BotConfig, Tag};

fn logged_in(session: &Session) -> bool {
    session.tap(|b| *b)
}

#[get("/")]
fn index(session: Session) -> Result<NamedFile, status::Custom<String>> {
    let login = logged_in(&session);
    println!("{}", login);
    if login {
        NamedFile::open("index.html").map_err(|e| Custom(Status::NotFound, e.to_string()))
    } else {
        Err(Custom(Status::Forbidden, "Forbidden".to_owned()))
    }
}

#[post("/login")]
fn login(session: Session) -> Result<rocket::response::Redirect, status::Custom<String>> {
    session.tap(|b| *b = true);
    Ok(rocket::response::Redirect::to("/"))
}

fn to_json_or_error<T>(v: &T) -> String
where
    T: serde::Serialize,
{
    let json = serde_any::to_string_pretty(&v, serde_any::format::Format::Json);
    match json {
        Ok(f) => return f,
        Err(e) => return format!("Error: {:#?}", &e),
    }
}

#[post("/add", data = "<tag>", format = "json")]
fn add_tag(
    session: Session,
    tag: Json<Tag>,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> String {
    let mut t = bc.write().unwrap();
    if logged_in(&session) {
        t.tags.push(tag.into_inner());
        write_config_logged(&config_file, &t);
        to_json_or_error(&t.tags)
    } else {
        "forbidden".to_string()
    }
}

#[get("/list")]
fn get_tags(session: Session, bc: rocket::State<'_, Arc<RwLock<BotConfig>>>) -> String {
    let t = bc.read().unwrap();
    if logged_in(&session) {
        to_json_or_error(&t.tags)
    } else {
        "forbidden".to_string()
    }
}

#[delete("/<id>")]
fn delete_tag(
    session: Session,
    id: usize,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> String {
    let mut t = bc.write().unwrap();
    if logged_in(&session) && id < t.tags.len() {
        let tag = t.tags.remove(id);
        write_config_logged(&config_file, &t);
        to_json_or_error(&tag)
    } else {
        return "forbidden".to_string();
    }
}

pub fn rocket(bc: Arc<RwLock<BotConfig>>, config_file: String) -> rocket::Rocket {
    rocket::ignite()
        .manage(bc)
        .manage(config_file)
        .mount("/", routes![index, login])
        .mount("/tags", routes![add_tag, delete_tag, get_tags])
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

    #[test]
    fn login() {
        let client = prepare_client();

        // not logged in.
        let mut response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Forbidden);
        assert_eq!(response.body_string(), Some("Forbidden".to_string()));

        // login
        let mut response = client.post("/login").dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.body_string(), None);

        // logged in
        let mut response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        if let Ok(s) = std::fs::read_to_string("index.html") {
            assert_eq!(response.body_string(), Some(s));
        } else {
            panic!("index.html not found");
        }
    }

    #[test]
    fn get_tags() {
        let (client, bc) = prepare_client_bc();

        //login
        let _ = client.post("/login").dispatch();

        let mut response = client.get("/tags/list").dispatch();
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
        let (client, bc) = prepare_client_bc();

        //login
        let _ = client.post("/login").dispatch();

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
        assert_eq!(response.status(), Status::Ok);
        // response data == server data
        assert_eq!(
            response.body_string().map(|mut s| s
                .drain(..)
                .filter(|c| *c != '\n' && *c != ' ')
                .collect::<String>()),
            serde_json::to_string(&bc.read().unwrap().tags).ok()
        );
        let new_count = bc.read().unwrap().tags.len();
        assert_eq!(new_count, old_count + 1);
    }

    #[test]
    fn delete_tag() {
        let (client, bc) = prepare_client_bc();

        // login
        let _ = client.post("/login").dispatch();

        // get old data
        let old_count = bc.read().unwrap().tags.len();
        assert!(old_count > 0);
        let old_tag = bc.read().unwrap().tags[old_count - 1].clone();

        // delete
        let mut response = client.delete(format!("/tags/{}", old_count - 1)).dispatch();
        assert_eq!(response.status(), Status::Ok);

        // check
        let new_count = bc.read().unwrap().tags.len();
        assert_eq!(new_count, old_count - 1);
        let removed_tag: crate::Tag = serde_json::from_str(&response.body_string().unwrap())
            .expect("Body content is not a json encoded Tag");
        assert_eq!(removed_tag.tag, old_tag.tag);
    }
}
