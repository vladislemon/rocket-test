#[macro_use]
extern crate rocket;

use std::env;

use anyhow::Result;
use rocket::fs::{FileServer, relative};
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;
use rocket::{Shutdown, State};
use rocket::time::OffsetDateTime;
use sqlx::{Pool, Postgres};
use sqlx::postgres::{PgPoolOptions};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct Message {
    id: i64,
    time: OffsetDateTime,
    text: String,
}

#[get("/")]
async fn index(pool: &State<Pool<Postgres>>) -> Result<String, Status> {
    let record = sqlx::query!("select name from test").fetch_one(pool.inner()).await;
    match record {
        Ok(record) => Ok(record.name),
        _ => Err(Status::NotFound),
    }
}

#[get("/message")]
async fn get_messages(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![Event + '_] {
    let mut rx = queue.subscribe();
    //let mut stream = sqlx::query_as!(Message, "select * from message").fetch(pool.inner());
    // EventStream! {
    //     loop {
    //         let next = stream.try_next().await;
    //         let message = match next {
    //                 Ok(option) => match option {
    //                 Some(message) => message,
    //                 None => break,
    //             },
    //             Err(_) => break,
    //         };
    //         yield Event::json(&message);
    //     }
    // }
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };
            yield Event::json(&msg);
        }
    }
}

#[post("/message", format = "json", data = "<message>")]
async fn send_message(pool: &State<Pool<Postgres>>, queue: &State<Sender<Message>>, message: Json<Message>) -> Status {
    let result = sqlx::query!(
        "insert into message (time, text) values ($1, $2)",
        OffsetDateTime::now_utc(),
        message.text
    ).execute(pool.inner()).await;
    let _ = queue.send(message.into_inner());
    match result {
        Ok(_) => Status::Ok,
        Err(_) => Status::InternalServerError
    }
}

#[rocket::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let database_url = env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await?;

    sqlx::migrate!().run(&pool).await?;

    rocket::build()
        .mount("/", routes![index, get_messages, send_message])
        .mount("/", FileServer::from(relative!("static")))
        .manage(pool)
        .manage(channel::<Message>(1024).0)
        .launch()
        .await?;

    Ok(())
}
