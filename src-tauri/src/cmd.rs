use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
  Subscribe { url: String },
  ExternalLink { url: String },
  Unsubscribe { id: i32 },
  Resubscribe { id: i32 },
}
