use super::models;
use super::schema::rss::dsl;
use ::rss::{Channel, Item};
use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;

const DB_PATH: &str = "./test.db";

pub fn connect_to_db() -> Result<SqliteConnection> {
  // need Ok(...?) to convert diesel::Error to anyhow::Error
  Ok(SqliteConnection::establish(DB_PATH)?)
}

pub async fn subscribe_to_feed(
  url: &str,
  connection: &SqliteConnection,
) -> Result<models::SendChannel> {
  let channel = fetch_channel(url).await?;
  let channel_model = add_feed(&channel, url, connection)?;
  for item in channel.into_items() {
    add_article(item, channel_model.id, connection)?;
  }
  Ok(models::SendChannel::from(&channel_model))
}
fn add_feed(
  channel: &Channel,
  url: &str,
  connection: &SqliteConnection,
) -> Result<models::Channel> {
  let pub_date = match channel.pub_date() {
    Some(date) => parse_rss_date(date)?,
    None => match channel.last_build_date() {
      Some(date) => parse_rss_date(date)?,
      None => diesel::select(now).first(connection)?,
    },
  };
  let new_channel = models::NewChannel {
    title: channel.title(),
    url,
    pub_date,
  };
  let feed_id = connection.transaction(|| -> Result<i32> {
    diesel::insert_or_ignore_into(dsl::rss)
      .values(new_channel)
      .execute(connection)?;
    Ok(
      dsl::rss
        .select(dsl::id)
        .filter(dsl::feed_id.is_null())
        .order(dsl::id.desc())
        .limit(1)
        .load(connection)?[0],
    )
  })?;
  Ok(models::Channel {
    id: feed_id,
    url: Some(channel.link().into()),
    pub_date,
    title: Some(channel.title().into()),
  })
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
    url: match item.link() {
      Some(url) => Some(url),
      None => match item.source() {
        Some(source) => Some(source.url()),
        None => None,
      },
    },
    title: match item.title() {
      Some(title) => title,
      None => "[Untitled Post]",
    },
    feed_id,
    content: match item.content() {
      None => match item.description() {
        Some(description) => description,
        None => "They didn't give us any content. Click the link to view article (hopefully 🥵).",
      },
      Some(content) => content,
    },
    pub_date: match item.pub_date() {
      Some(date) => parse_rss_date(date)?,
      None => diesel::select(now).first(connection)?,
    },
    read: false,
  };
  diesel::insert_or_ignore_into(dsl::rss)
    .values(new_item)
    .execute(connection)?;
  Ok(())
}
pub fn delete_all_feeds(connection: &SqliteConnection) -> Result<()> {
  diesel::delete(dsl::rss).execute(connection)?;
  Ok(())
}
pub fn send_items_by_feed(
  feed_id: i32,
  connection: &SqliteConnection,
) -> Result<Vec<models::SendItem>> {
  Ok(
    get_items_by_feed(feed_id, connection)?
      .iter()
      .map(models::SendItem::from)
      .collect(),
  )
}
fn get_items_by_feed(
  feed_id: i32,
  connection: &SqliteConnection,
) -> Result<Vec<models::Item>> {
  Ok(
    dsl::rss
      .select((
        dsl::id,
        dsl::url.nullable(),
        dsl::feed_id,
        dsl::read,
        dsl::pub_date,
        dsl::content,
        dsl::title.nullable(),
      ))
      .filter(dsl::feed_id.eq(feed_id))
      .order(dsl::id.desc())
      .load(connection)?,
  )
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
  add_new_articles_to_db(feed.id, updated_feed, connection)?;
  Ok(())
}

fn add_new_articles_to_db(
  feed_id: i32,
  channel: Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  let newest_article_date: NaiveDateTime = dsl::rss
    .select(dsl::pub_date)
    .order(dsl::pub_date)
    .limit(1)
    .load(connection)?[0];
  let old_items = get_items_by_feed(feed_id, connection)?;
  for item in channel.into_items() {
    if item_is_not_in_db(&item, newest_article_date, &old_items)? {
      add_article(item, feed_id, connection)?;
    }
  }
  Ok(())
}

fn item_is_not_in_db(
  item: &Item,
  newest_article_date: NaiveDateTime,
  old_items: &[models::Item],
) -> Result<bool> {
  if let Some(date) = item.pub_date() {
    Ok(parse_rss_date(date)? > newest_article_date)
  } else if let Some(title) = item.title() {
    Ok(
      old_items
        .iter()
        .any(|item| item.title != Some(title.into())),
    )
  } else {
    Ok(true)
  }
}

pub fn send_all_feeds(
  connection: &SqliteConnection,
) -> Result<Vec<models::SendChannel>> {
  Ok(
    get_all_feeds(connection)?
      .iter()
      .map(models::SendChannel::from)
      .collect(),
  )
}

fn get_all_feeds(
  connection: &SqliteConnection,
) -> Result<Vec<models::Channel>> {
  Ok(
    dsl::rss
      .select((
        dsl::id,
        dsl::url.nullable(),
        dsl::pub_date,
        dsl::title.nullable(),
      ))
      .filter(dsl::feed_id.is_null())
      .load::<models::Channel>(connection)?,
  )
}

pub fn unsubscribe(feed_id: i32, connection: &SqliteConnection) -> Result<()> {
  diesel::delete(dsl::rss.filter(dsl::id.eq(feed_id))).execute(connection)?;
  Ok(())
}

pub fn remove_stories_from_unsubbed_feed(
  feed_id: i32,
  connection: &SqliteConnection,
) -> Result<()> {
  diesel::delete(dsl::rss.filter(dsl::feed_id.eq(feed_id)))
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
