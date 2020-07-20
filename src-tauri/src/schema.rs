table! {
    links (id) {
        id -> Integer,
        item -> Integer,
        tag -> Integer,
    }
}

table! {
    rss (id) {
        id -> Integer,
        url -> Text,
        feed_id -> Nullable<Integer>,
        read -> Nullable<Bool>,
        pub_date -> Timestamp,
        content -> Nullable<Text>,
        title -> Nullable<Text>,
    }
}

table! {
    tags (id) {
        id -> Integer,
        name -> Text,
    }
}

joinable!(links -> rss (item));
joinable!(links -> tags (tag));

allow_tables_to_appear_in_same_query!(
    links,
    rss,
    tags,
);
