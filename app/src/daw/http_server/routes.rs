use std::{
    io::Read,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tiny_http::Request;

use super::{DawHttpCommand, DawHttpCommandKind, DawHttpState, DawStatusSnapshot};
use crate::daw::{CacheState, DawPlayState};

const MAX_JSON_BODY_BYTES: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[path = "routes/gets.rs"]
mod gets;
#[path = "routes/posts.rs"]
mod posts;
#[path = "routes/response.rs"]
mod response;
#[path = "routes/snapshot.rs"]
mod snapshot;

pub(super) use gets::{handle_get_mml, handle_get_mmls, handle_get_patches, handle_get_status};
pub(super) use posts::{
    handle_options, handle_post_ab_repeat, handle_post_mixer, handle_post_mml,
    handle_post_mode_daw, handle_post_patch, handle_post_patch_random, handle_post_play_start,
    handle_post_play_stop,
};
use response::{empty_response, json_response, with_etag_header};
#[cfg(test)]
pub(super) use response::{is_allowed_cors_origin, request_origin};
pub(super) use response::{
    request_header_value, text_response, validate_cors_request, with_cors_headers,
    with_preflight_cors_headers, RequestHeaderName,
};
pub(super) use snapshot::{
    get_snapshot_mml, get_snapshot_mmls, get_status_snapshot, if_none_match_matches,
    parse_get_mml_query, snapshot_mmls_etag,
};
