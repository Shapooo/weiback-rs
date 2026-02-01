-- Add migration script here
CREATE TABLE
    posts (
        attitudes_count INTEGER,
        attitudes_status INTEGER,
        comments_count INTEGER,
        created_at TEXT,
        deleted BOOLEAN,
        edit_count INTEGER,
        favorited BOOLEAN,
        geo TEXT,
        id INTEGER PRIMARY KEY,
        mblogid TEXT,
        mix_media_ids TEXT,
        mix_media_info TEXT,
        page_info TEXT,
        pic_ids TEXT,
        pic_infos TEXT,
        pic_num INTEGER,
        region_name TEXT,
        reposts_count INTEGER,
        repost_type INTEGER,
        retweeted_id INTEGER,
        source TEXT,
        text TEXT,
        uid INTEGER,
        url_struct TEXT
    );

CREATE TABLE
    favorited_posts (id INTEGER PRIMARY KEY, unfavorited BOOLEAN);

CREATE TABLE
    users (
        avatar_hd TEXT,
        avatar_large TEXT,
        domain TEXT,
        following BOOLEAN,
        follow_me BOOLEAN,
        id INTEGER PRIMARY KEY,
        profile_image_url TEXT,
        screen_name TEXT
    );

CREATE TABLE
    picture (
        id TEXT,
        definition TEXT,
        path TEXT,
        post_id TEXT,
        url TEXT PRIMARY KEY,
        user_id TEXT
    );

CREATE TABLE
    video (url TEXT PRIMARY KEY, path TEXT, post_id TEXT);
