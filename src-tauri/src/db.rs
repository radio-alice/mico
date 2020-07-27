use super::models;
use super::schema::rss::dsl;
use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use futures::future::try_join_all;
use std::str::FromStr;

const DB_PATH: &str = "./test.db";

pub fn connect_to_db() -> Result<SqliteConnection> {
  Ok(SqliteConnection::establish(DB_PATH)?)
}

pub async fn subscribe_to_feed(
  url: &str,
  connection: &SqliteConnection,
) -> Result<models::SendChannel> {
  let channel_xml = surf::get(format_url(url))
    .recv_bytes()
    .await
    .map_err(Error::msg)?;
  let rss_channel = rss::Channel::read_from(&channel_xml[..])?;
  let channel_model = add_feed(&rss_channel, url, connection)?;
  let new_items = rss_channel
    .into_items()
    .iter()
    .map(|item| new_item(item, channel_model.id, connection))
    .collect::<Vec<models::NewItem>>();
  diesel::insert_or_ignore_into(dsl::rss)
    .values(&new_items)
    .execute(connection)?;
  Ok(models::SendChannel::from(&channel_model))
}
fn add_feed(
  channel: &rss::Channel,
  url: &str,
  connection: &SqliteConnection,
) -> Result<models::Channel> {
  let pub_date_str = channel.pub_date().or_else(|| {
    channel.last_build_date().or_else(|| {
      channel
        .dublin_core_ext()
        .map(|dc_ext| &dc_ext.dates()[0][..])
    })
  });

  let parsed_date = parse_rss_date(pub_date_str)
    .unwrap_or(diesel::select(now).first(connection)?);

  let new_channel = models::NewChannel {
    title: channel.title(),
    url,
    pub_date: parsed_date,
  };
  let feed_id = connection.transaction(|| -> Result<i32> {
    diesel::insert_or_ignore_into(dsl::rss)
      .values(new_channel)
      .execute(connection)?;
    Ok(
      dsl::rss
        .select(dsl::id)
        .filter(dsl::feed_id.is_null())
        .filter(dsl::title.eq(channel.title()))
        .limit(1)
        .load(connection)?[0],
    )
  })?;
  Ok(models::Channel {
    id: feed_id,
    url: Some(channel.link().into()),
    pub_date: parsed_date,
    title: Some(channel.title().into()),
  })
}

fn new_item(
  item: &rss::Item,
  feed_id: i32,
  connection: &SqliteConnection,
) -> models::NewItem {
  let pub_date_str = item
    .pub_date()
    .or_else(|| item.dublin_core_ext().map(|dc_ext| &dc_ext.dates()[0][..]));

  // not sure how to avoid .unwrap here
  let parsed_date = parse_rss_date(pub_date_str)
    .unwrap_or_else(|| diesel::select(now).first(connection).unwrap());

  // concat possible media embed with content (no idea why it won't fmt)
  let media =
    create_enclosure_html(item.enclosure()).unwrap_or_else(|| "".to_string());
  let content = media + item.content()
                            .or_else(|| item.description())
                            .unwrap_or("They didn't give us any content. Click the link to view article (hopefully 🥵).");

  let url = item
    .link()
    .map(|link| link.to_owned())
    .or_else(|| item.source().map(|source| source.url().to_owned()));

  models::NewItem {
    url,
    title: item.title().unwrap_or("[Untitled Post]").to_owned(),
    feed_id,
    content,
    pub_date: parsed_date,
    read: false,
  }
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
  // TODO use a thread pool here?
  // TODO handle offline elegantly
  // handle all these requests together so that we can join the asyncs together
  let all_refresh_requests = all_channels.iter().map(|feed| {
    surf::get(format_url(
      feed
        .url
        .as_ref()
        .expect("somehow stored a feed with no url"),
    ))
    .recv_bytes()
  });
  let all_new_xml = try_join_all(all_refresh_requests)
    .await
    .map_err(Error::msg)?;
  let all_new_channels = all_new_xml
    .iter()
    .map(|feed_xml| rss::Channel::read_from(&feed_xml[..]));
  for (channel, new_channel) in all_channels.iter().zip(all_new_channels) {
    refresh_feed(channel, new_channel?, connection)?;
  }
  Ok(())
}
fn refresh_feed(
  feed: &models::Channel,
  new_feed: rss::Channel,
  connection: &SqliteConnection,
) -> Result<()> {
  // check if we can tell it *doesn't* need a refresh
  if let Some(date) =
    parse_rss_date(new_feed.pub_date().or_else(|| new_feed.last_build_date()))
  {
    if date <= feed.pub_date {
      return Ok(());
    }
  };
  // needs refresh
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
  let new_items = channel
    .into_items()
    .iter()
    .filter(|item| item_is_not_in_db(&item, newest_article_date, &old_items))
    .map(|item| new_item(&item, feed_id, connection))
    .collect::<Vec<models::NewItem>>();
  diesel::insert_or_ignore_into(dsl::rss)
    .values(&new_items)
    .execute(connection)?;
  Ok(())
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

fn parse_rss_date(maybe_date: Option<&str>) -> Option<NaiveDateTime> {
  // TODO - make this smarter
  maybe_date
    .map(|date| {
      NaiveDateTime::parse_from_str(date, "%a, %d %b %Y %T %z")
        .ok()
        .or_else(|| {
          NaiveDateTime::parse_from_str(date, "%a, %d %b %Y %T UT")
            .ok()
            .or_else(|| {
              NaiveDateTime::parse_from_str(date, "%a, %d %b %Y %T GMT")
                .ok()
                .or_else(|| {
                  NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%:z")
                    .ok()
                    .or_else(|| NaiveDateTime::from_str(date).ok())
                })
            })
        })
    })
    .flatten()
}
fn create_enclosure_html(
  maybe_media: Option<&rss::Enclosure>,
) -> Option<String> {
  maybe_media
    .map(|media| {
      match media.mime_type().split('/').collect::<Vec<&str>>().first() {
        Some(&"audio") => {
          Some(format!("<audio src=\"{}\" controls></audio>", media.url()))
        }
        Some(&"video") => {
          Some(format!("<video src=\"{}\" controls></video>", media.url()))
        }
        Some(&"image") => Some(format!("<img src=\"{}\">", media.url())),
        _ => None,
      }
    })
    .flatten()
}
fn item_is_not_in_db(
  item: &rss::Item,
  newest_article_date: NaiveDateTime,
  old_items: &[models::Item],
) -> bool {
  if let Some(date) = parse_rss_date(item.pub_date()) {
    date > newest_article_date
  } else if let Some(title) = item.title() {
    old_items
      .iter()
      .any(|item| item.title != Some(title.into()))
  } else {
    true
  }
}

fn format_url(url: &str) -> String {
  if !url.starts_with("http") {
    return format!("https://{}", url);
  }
  url.to_string()
}
