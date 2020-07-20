use super::models;
use super::schema::rss as rss_table;
use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use diesel::dsl::{exists, now};
use diesel::prelude::*;
use rss::{Channel, Item};

const DB_PATH: &str = "./test.db";

pub fn connect_to_db() -> Result<SqliteConnection> {
  // need Ok(...?) to convert diesel::Error to anyhow::Error
  Ok(SqliteConnection::establish(DB_PATH)?)
}

pub fn subscribe_to_feed(
  channel: Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  let feed_id = add_feed(channel, connection)?;
  for item in channel.into_items() {
    add_article(item, feed_id, connection)?;
  }
  Ok(())
}
fn add_feed(channel: Channel, connection: &SqliteConnection) -> Result<i32> {
  let pub_date = match channel.pub_date() {
    Some(date) => parse_rss_date(date)?,
    None => diesel::select(now).first(connection)?,
  };
  let new_channel = models::NewChannel {
    title: channel.title(),
    url: channel.link(),
    pub_date,
  };
  Ok(connection.transaction(|| -> Result<i32> {
    diesel::insert_or_ignore_into(rss_table::table)
      .values(new_channel)
      .execute(connection)?;
    Ok(
      rss_table::table
        .select(rss_table::id)
        .order(rss_table::id.desc())
        .limit(1)
        .load(connection)?[0],
    )
  })?)
}
fn parse_rss_date(date: &str) -> Result<NaiveDateTime> {
  Ok(NaiveDateTime::parse_from_str(date, "%a, %d %b %Y %T %z")?)
}
fn add_article(
  item: Item,
  feed_id: i32,
  connection: &SqliteConnection,
) -> Result<()> {
  let new_item = models::NewItem {
    url: item.link(),
    title: item.title(),
    feed_id,
    content: match item.content() {
      None => match item.description(){
        Some(description) => description,
        None => "They didn't give us any content. Click the link to view article (hopefully 🥵)."
      },
      Some(content) => content,
    },
    pub_date: match item.pub_date() {
      Some(date) => parse_rss_date(date)?,
    None => diesel::select(now).first(connection)?,
    },
    read: false,
  };
  diesel::insert_or_ignore_into(rss_table::table)
    .values(new_item)
    .execute(connection)?;
  Ok(())
}

pub async fn refresh_all_feeds(connection: &SqliteConnection) -> Result<()> {
  for feed in get_all_feeds(connection)? {
    refresh_feed(feed, connection).await?;
  }
  Ok(())
}
async fn refresh_feed(
  feed: models::Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  let updated_feed =
    fetch_channel(&feed.url.expect("somehow stored a feed with no url"))
      .await?;
  // check if we can tell it *doesn't* need a refresh
  match (updated_feed.pub_date(), updated_feed.last_build_date()) {
    (Some(date), _) | (_, Some(date)) => {
      if parse_rss_date(date)? <= feed.pub_date {
        return Ok(());
      }
    }

    // needs a refresh
    (_, _) => (),
  };

  add_new_articles_to_db(feed.id, updated_feed, connection);

  Ok(())
}

fn add_new_articles_to_db(
  feed_id: i32,
  channel: Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  let newest_article_date: NaiveDateTime = rss_table::table
    .select(rss_table::pub_date)
    .order(rss_table::pub_date)
    .limit(1)
    .load(connection)?[0];

  for item in channel.into_items() {
    if item_is_not_in_db(&item, newest_article_date, connection)? {
      add_article(item, feed_id, connection);
    }
  }
  Ok(())
}

fn item_is_not_in_db(
  item: &Item,
  newest_article_date: NaiveDateTime,
  connection: &SqliteConnection,
) -> Result<bool> {
  if let Some(date) = item.pub_date() {
    Ok(parse_rss_date(date)? > newest_article_date)
  } else if let Some(title) = item.title() {
    Ok(
      diesel::select(exists(
        rss_table::table.filter(rss_table::title.eq(title)),
      ))
      .get_result(connection)?,
    )
  } else {
    Ok(true)
  }
}

fn get_all_feeds(
  connection: &SqliteConnection,
) -> Result<Vec<models::Channel>> {
  Ok(
    rss_table::table
      .select((
        rss_table::id,
        rss_table::url.nullable(),
        rss_table::pub_date,
        rss_table::title.nullable(),
      ))
      .filter(rss_table::feed_id.eq(None: Option<i32>))
      .load::<models::Channel>(connection)?,
  )
}

pub fn unsubscribe(feed_id: i32, connection: &SqliteConnection) -> Result<()> {
  diesel::delete(rss_table::table.filter(rss_table::id.eq(feed_id)))
    .execute(connection)?;
  Ok(())
}

pub fn remove_stories_from_unsubbed_feed(
  feed_id: i32,
  connection: &SqliteConnection,
) -> Result<()> {
  diesel::delete(rss_table::table.filter(rss_table::feed_id.eq(feed_id)))
    .execute(connection)?;
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
