use super::models;
use super::schema::rss::dsl;
use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use futures::future::try_join_all;

const DB_PATH: &str = "./test.db";

pub fn connect_to_db() -> Result<SqliteConnection> {
  // need Ok(...?) to convert diesel::Error to anyhow::Error
  Ok(SqliteConnection::establish(DB_PATH)?)
}

pub async fn subscribe_to_feed(
  url: &str,
  connection: &SqliteConnection,
) -> Result<models::SendChannel> {
  let channel_xml = surf::get(url).recv_bytes().await.map_err(Error::msg)?;
  let rss_channel = rss::Channel::read_from(&channel_xml[..])?;
  let channel_model = add_feed(&rss_channel, url, connection)?;
  for item in rss_channel.into_items() {
    add_article(item, channel_model.id, connection)?;
  }
  Ok(models::SendChannel::from(&channel_model))
}
fn add_feed(
  channel: &rss::Channel,
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
        .order(dsl::pub_date.desc())
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
  item: rss::Item,
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
pub fn delete_all_channels(connection: &SqliteConnection) -> Result<()> {
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
      .order(dsl::pub_date.desc())
      .load(connection)?,
  )
}
pub async fn refresh_all_channels(connection: &SqliteConnection) -> Result<()> {
  let all_channels = get_all_channels(connection)?;
  // handle all these requests together so that we can join the asyncs together
  // TODO use a thread pool here?
  // TODO handle offline elegantly
  let all_refresh_requests = all_channels.iter().map(|feed| {
    surf::get(
      feed
        .url
        .as_ref()
        .expect("somehow stored a feed with no url"),
    )
    .recv_bytes()
  });
  let all_new_xml = try_join_all(all_refresh_requests)
    .await
    .map_err(Error::msg)?;
  let all_new_channels = all_new_xml
    .iter()
    .map(|feed_xml| rss::Channel::read_from(&feed_xml[..]));
  for (channel, new_channel) in all_channels.iter().zip(all_new_channels) {
    refresh_feed(channel, new_channel?, connection);
  }
  Ok(())
}
fn refresh_feed(
  feed: &models::Channel,
  new_feed: rss::Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  // check if we can tell it *doesn't* need a refresh
  match (new_feed.pub_date(), new_feed.last_build_date()) {
    (Some(date), _) | (_, Some(date)) => {
      if parse_rss_date(date)? <= feed.pub_date {
        return Ok(());
      }
    }

    // needs a refresh
    (_, _) => (),
  };
  add_new_articles_to_db(feed.id, new_feed, connection)?;
  Ok(())
}

fn add_new_articles_to_db(
  feed_id: i32,
  channel: rss::Channel,
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
  item: &rss::Item,
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

pub fn send_all_channels(
  connection: &SqliteConnection,
) -> Result<Vec<models::SendChannel>> {
  Ok(
    get_all_channels(connection)?
      .iter()
      .map(models::SendChannel::from)
      .collect(),
  )
}

fn get_all_channels(
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
pub fn send_all_items(
  connection: &SqliteConnection,
) -> Result<Vec<models::SendItem>> {
  Ok(
    get_all_items(connection)?
      .iter()
      .map(models::SendItem::from)
      .collect(),
  )
}
fn get_all_items(connection: &SqliteConnection) -> Result<Vec<models::Item>> {
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
      .filter(dsl::feed_id.is_not_null())
      .order(dsl::pub_date.desc())
      .load(connection)?,
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
