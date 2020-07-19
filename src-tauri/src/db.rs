use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use rss::{Channel, Item};
use rusqlite::{params, Connection, NO_PARAMS};
const DB_PATH: &'static str = "../test.db";

pub fn setup_db() -> Result<()> {
  let connection = connect_to_db()?;
  connection.execute(
    "CREATE TABLE IF NOT EXISTS rss (
        id INTEGER PRIMARY KEY,
        url TEXT NOT NULL UNIQUE,
        feed_id INTEGER,
        read BOOLEAN,
        pub_date TEXT,
        content TEXT,
        title TEXT UNIQUE,
      )",
    NO_PARAMS,
  )?;

  connection.execute(
    "CREATE TABLE IF NOT EXISTS tags (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL
        )",
    NO_PARAMS,
  )?;

  connection.execute(
    "CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY,
    item INTEGER FOREIGN KEY NOT NULL,
    tag INTEGER FOREIGN KEY NOT NULL
    ",
    NO_PARAMS,
  )?;

  Ok(())
}

pub fn connect_to_db() -> Result<Connection> {
  // need Ok(...?) to convert rusqlite::Error to anyhow::Error
  Ok(Connection::open(DB_PATH)?)
}

pub fn subscribe_to_feed(
  channel: Channel,
  connection: &Connection,
) -> Result<()> {
  let feed_id = add_feed(channel, connection)?;
  for item in channel.into_items() {
    add_article(item, feed_id, connection)?;
  }
  Ok(())
}
fn add_feed(channel: Channel, connection: &Connection) -> Result<i64> {
  let pub_date = match channel.pub_date() {
    Some(date) => DateTime::from(DateTime::parse_from_rfc2822(date)?),
    None => Utc::now(),
  };
  connection.execute(
    "INSERT INTO rss (url, title, pub_date)
         VALUES ($1, $2, $3)",
    params![channel.link(), channel.title(), pub_date],
  )?;
  Ok(connection.last_insert_rowid())
}

fn add_article(
  item: Item,
  feed_id: i64,
  connection: &Connection,
) -> Result<()> {
  let url = item.link();
  let title = item.title();
  let content = match item.content() {
      None => match item.description(){
        Some(description) => description,
        None => "They didn't give us any content :/. Click the link to view article (I hope)."
      },
      Some(content) => content,
    };
  let pub_date = match item.pub_date() {
    Some(date) => DateTime::from(DateTime::parse_from_rfc2822(date)?),
    None => Utc::now(),
  };
  // can't figure out a way to do this programmatically
  match (url, title) {
    (Some(url), Some(title)) => connection.execute(
      "INSERT OR IGNORE INTO rss (feed_id, url, title, content, pub_date, read)
    VALUES ($1, $2, $3, $4, $5, $6)",
      params![feed_id, url, title, content, pub_date, false],
    )?,
    (None, Some(title)) => connection.execute(
      "INSERT OR IGNORE INTO rss (feed_id, title, content, pub_date, read)
    VALUES ($1, $2, $3, $4, $5)",
      params![feed_id, title, content, pub_date, false],
    )?,
    (Some(url), None) => connection.execute(
      "INSERT OR IGNORE INTO rss (feed_id, url, content, pub_date, read)
    VALUES ($1, $2, $3, $4, $5)",
      params![feed_id, url, content, pub_date, false],
    )?,
    (None, None) => connection.execute(
      "INSERT OR IGNORE INTO rss (feed_id, content, pub_date, read)
    VALUES ($1, $2, $3, $4)",
      params![feed_id, content, pub_date, false],
    )?,
  };
  Ok(())
}

struct FeedToRefresh {
  id: i64,
  url: String,
  pub_date: DateTime<Utc>,
}
pub async fn refresh_all_feeds(connection: &Connection) -> Result<()> {
  for feed in get_all_feeds(connection)? {
    refresh_feed(feed, connection).await?;
  }
  Ok(())
}
async fn refresh_feed(
  feed: FeedToRefresh,
  connection: &Connection,
) -> Result<()> {
  let updated_feed = fetch_channel(&feed.url).await?;
  // check if we can tell it *doesn't* need a refresh
  match (updated_feed.pub_date(), updated_feed.last_build_date()) {
    (Some(date), _) | (_, Some(date)) => {
      if DateTime::from(DateTime::parse_from_rfc2822(date)?): DateTime<Utc>
        <= feed.pub_date
      {
        return Ok(());
      }
    }
    (None, None) => (),
  };

  add_new_articles_to_db(feed.id, updated_feed, connection);

  Ok(())
}

fn add_new_articles_to_db(
  feed_id: i64,
  channel: Channel,
  connection: &Connection,
) -> Result<()> {
  let newest_article_date: DateTime<Utc> = connection.query_row(
    "SELECT pub_date WHERE (feed_id=$1) FROM rss LIMIT 1 ORDER BY pub_date",
    params![feed_id],
    |row| row.get(1),
  )?;
  for item in channel.into_items() {
    if item_is_not_in_db(&item, newest_article_date, connection)? {
      add_article(item, feed_id, connection);
    }
  }
  Ok(())
}

fn item_is_not_in_db(
  item: &Item,
  newest_article_date: DateTime<Utc>,
  connection: &Connection,
) -> Result<bool> {
  if let Some(date) = item.pub_date() {
    Ok(
      DateTime::from(DateTime::parse_from_rfc2822(date)?): DateTime<Utc>
        > newest_article_date,
    )
  } else if let Some(title) = item.title() {
    Ok(
      !connection
        .prepare("SELECT EXISTS(SELECT 1 FROM rss WHERE (title=$1))")?
        .exists(params![title])?,
    )
  } else {
    Ok(true)
  }
}

fn get_all_feeds(connection: &Connection) -> Result<Vec<FeedToRefresh>> {
  let mut all_feeds_query =
    connection.prepare("SELECT id, url, pub_date FROM feeds")?;
  let all_rows = all_feeds_query.query_and_then(
    NO_PARAMS,
    |row| -> Result<FeedToRefresh> {
      Ok(FeedToRefresh {
        id: row.get(0)?,
        url: row.get(1)?,
        pub_date: row.get(2)?,
      })
    },
  )?;
  let mut feeds = Vec::new();
  for row in all_rows {
    feeds.push(row?);
  }
  Ok(feeds)
}

pub fn unsubscribe(feed_id: i64, connection: &Connection) -> Result<()> {
  connection.execute("DELETE FROM rss WHERE (id=$1)", params![feed_id])?;
  Ok(())
}

pub fn remove_stories_from_unsubbed_feed(
  feed_id: i64,
  connection: &Connection,
) -> Result<()> {
  connection.execute("DELETE FROM rss WHERE (feed_id=$1)", params![feed_id])?;
  Ok(())
}

// TODO add tags to feeds
// TODO add tags to articles
// TODO rm tags from feeds
// TODO rm tags from articles

fn format_url(url: &str) -> String {
  if !url.starts_with("http") {
    return format!("https://{}", url);
  }
  url.to_string()
}
async fn fetch_channel(url: &str) -> Result<Channel> {
  let feed_xml = surf::get(format_url(url))
    .recv_bytes()
    .await
    .map_err(Error::msg)?;
  let channel = Channel::read_from(&feed_xml[..])?;
  Ok(channel)
}
