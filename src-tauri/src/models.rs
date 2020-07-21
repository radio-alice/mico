use super::schema::{links, rss, tags};
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};
use serde::Serialize;

#[derive(Queryable)]
pub struct Channel {
  pub id: i32,
  pub url: Option<String>,
  pub pub_date: NaiveDateTime,
  pub title: Option<String>,
}

#[derive(Insertable)]
#[table_name = "rss"]
pub struct NewChannel<'a> {
  pub url: &'a str,
  pub pub_date: NaiveDateTime,
  pub title: &'a str,
}

#[derive(Serialize)]
pub struct SendChannel {
  pub id: i32,
  pub url: String,
  pub date: String,
  pub title: String,
}

impl SendChannel {
  pub fn from(channel: &Channel) -> Self {
    // `.expect`s below are reasonable due to valid channels requiring title + url
    SendChannel {
      id: channel.id,
      url: channel
        .url
        .as_ref()
        .expect("somehow got a channel w no url")
        .into(),
      date: channel.pub_date.format("%m-%d-%Y").to_string(),
      title: channel
        .title
        .as_ref()
        .expect("somehow got a channel w no title")
        .into(),
    }
  }
}

#[derive(Queryable)]
pub struct Item {
  pub id: i32,
  pub url: Option<String>,
  pub feed_id: Option<i32>,
  pub read: Option<bool>,
  pub pub_date: NaiveDateTime,
  pub content: Option<String>,
  pub title: Option<String>,
}

#[derive(Insertable)]
#[table_name = "rss"]
pub struct NewItem<'a> {
  pub url: Option<&'a str>,
  pub feed_id: i32,
  pub read: bool,
  pub pub_date: NaiveDateTime,
  pub content: &'a str,
  pub title: &'a str,
}

#[derive(Serialize)]
pub struct SendItem {
  pub id: i32,
  pub url: Option<String>,
  pub feed_id: i32,
  pub read: bool,
  pub date: String,
  pub content: String,
  pub title: String,
}
impl SendItem {
  pub fn from(item: &Item) -> Self {
    // .expects reasonable as these fields are created with item
    SendItem {
      id: item.id,
      url: item.url.clone(),
      feed_id: item.feed_id.expect("article with no feed id???"),
      read: item.read.unwrap_or(false),
      date: item.pub_date.format("%m-%d-%Y").to_string(),
      content: item
        .content
        .as_ref()
        .unwrap_or(&"[no content found for this post]".into())
        .into(),
      title: item
        .title
        .as_ref()
        .unwrap_or(&"[Untitled Post]".into())
        .into(),
    }
  }
}
#[derive(Queryable)]
pub struct Tag<'a> {
  pub id: i32,
  pub name: &'a str,
}

#[derive(Queryable)]
pub struct Link {
  pub id: i32,
  pub item_id: i32,
  pub tag_id: i32,
}
