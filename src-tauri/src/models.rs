use super::schema::{links, rss, tags};
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};

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

#[derive(Queryable)]
pub struct Item {
  pub id: i32,
  pub url: Option<String>,
  pub feed_id: i32,
  pub read: bool,
  pub pub_date: NaiveDateTime,
  pub content: String,
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
  pub title: Option<&'a str>,
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
