use bytes::Bytes;
use serde_json::Value;
// use std::collections:Vec
struct post {
    json: Value,
    pics: Vec<Bytes>,
}
