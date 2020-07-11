use anyhow::Result;
use rusqlite::{params, Connection, NO_PARAMS};
use serde::Serialize;
use std::collections::HashSet;
const DB_PATH: &'static str = "../test.db";

// TODO FIGURE OUT HOW TO QUERY BY TAG
// current setup won't work
// need some way of getting an array into sqlite I think
// bc tags - articles/feeds are a many-to-many relationship

#[derive(Serialize, Debug)]
pub struct Feed {
  pub id: i64,
  pub url: String,
  pub tags: HashSet<String>,
}
// TODO add Article struct

pub fn setup_db() -> Result<()> {
  let connection = connect_to_db()?;
  connection.execute(
    "CREATE TABLE IF NOT EXISTS feeds (
        id INTEGER PRIMARY KEY,
        url TEXT NOT NULL UNIQUE,
        tags BLOB)",
    NO_PARAMS,
  )?;

  connection.execute(
    "CREATE TABLE IF NOT EXISTS articles (
        id INTEGER PRIMARY KEY,
        feed_id INTEGER NOT NULL,
        read BOOLEAN NOT NULL,
        tags BLOB)",
    NO_PARAMS,
  )?;

  Ok(())
}

pub fn connect_to_db() -> Result<Connection> {
  // need Ok(...?) to convert rusqlite::Error to anyhow::Error
  Ok(Connection::open(DB_PATH)?)
}

// TODO add articles to db when feed is added
pub fn add_feed(
  url: &String,
  tags: &HashSet<String>,
  connection: &Connection,
) -> Result<i64> {
  let serialized_tags = bincode::serialize(tags)?;
  connection.execute(
    "INSERT INTO feeds (url, tags)
         VALUES ($1, $2)",
    params![url, serialized_tags],
  )?;
  Ok(connection.last_insert_rowid())
}

pub fn get_feeds(connection: &Connection) -> Result<Vec<Feed>> {
  let mut all_feeds_query =
    connection.prepare("SELECT id, url, tags FROM feeds")?;
  let all_rows =
    all_feeds_query.query_and_then(NO_PARAMS, |row| -> Result<Feed> {
      Ok(Feed {
        id: row.get(0)?,
        url: row.get(1)?,
        tags: bincode::deserialize(&row.get::<_, Vec<u8>>(2)?[..])?,
      })
    })?;
  let mut feeds = Vec::new();
  for row in all_rows {
    feeds.push(row?);
  }
  Ok(feeds)
}

// TODO impl unsubscribing from feeds

// TODO add tags to feeds
// TODO add tags to articles
// TODO rm tags from feeds
// TODO rm tags from articles
