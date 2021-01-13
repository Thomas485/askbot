use std::sync::{Arc, RwLock};

use rocket::response::status;
use rocket::response::NamedFile;

use crate::{write_config, BotConfig, Tag};

#[get("/<key>")]
fn index(key: String) -> Result<NamedFile, status::NotFound<String>> {
    NamedFile::open("index.html").map_err(|e| status::NotFound(e.to_string()))
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

#[get("/add_tag/<tag>/<webhook>/<key>")]
fn add_tag(
    tag: String,
    webhook: String,
    key: String,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> String {
    let mut t = bc.write().unwrap();
    if key == t.key {
        let new_tag = Tag { tag, webhook };
        t.tags.push(new_tag);
        write_config(&config_file, &t);
        to_json_or_error(&t.tags)
    } else {
        "forbidden".to_string()
    }
}

#[get("/tags/<key>")]
fn get_tags(key: String, bc: rocket::State<'_, Arc<RwLock<BotConfig>>>) -> String {
    let t = bc.write().unwrap();
    if key == t.key {
        to_json_or_error(&t.tags)
    } else {
        "forbidden".to_string()
    }
}

#[get("/delete_tag/<id>/<key>")]
fn delete_tag(
    id: usize,
    key: String,
    bc: rocket::State<'_, Arc<RwLock<BotConfig>>>,
    config_file: rocket::State<String>,
) -> String {
    let mut t = bc.write().unwrap();
    if key == t.key && id < t.tags.len() {
        let tag = t.tags.remove(id);
        write_config(&config_file, &t);
        to_json_or_error(&tag)
    } else {
        return "forbidden".to_string();
    }
}

pub fn rocket_main(bc: Arc<RwLock<BotConfig>>, config_file: String) {
    rocket::ignite()
        .manage(bc)
        .manage(config_file)
        .mount("/", routes![index, add_tag, delete_tag, get_tags])
        .launch();
}
