pub mod coverart;
pub mod playback;
pub mod queue;
pub mod song;

pub mod endpoints {
    pub const ROOT: &str = "/";

    pub const CREATESONG: &str = "/api/v2/song";
    pub const GETSONGS: &str = "/api/v2/song";
    pub const GETALLSONGS: &str = "/api/v2/song/all";
    pub const STREAMSONG: &str = "/api/v2/song/stream/{id}";
    pub const DOWNLOADSONG: &str = "/api/v2/song/download/{id}";
    pub const DELETESONG: &str = "/api/v2/song/{id}";
    pub const CREATECOVERART: &str = "/api/v2/coverart";
    pub const GETCOVERART: &str = "/api/v2/coverart";
    pub const DOWNLOADCOVERART: &str = "/api/v2/coverart/download/{id}";

    pub const CREATEPLAYBACKQUEUE: &str = "/api/v2/playback/queue";
    pub const GETPLAYBACKQUEUE: &str = "/api/v2/playback/queue";
    pub const ADDPLAYBACKQUEUE: &str = "/api/v2/playback/queue/add";
    pub const GETPLAYBACKQUEUEITEM: &str = "/api/v2/playback/queue/item";
    pub const CLEARPLAYBACKQUEUE: &str = "/api/v2/playback/queue/clear";
}

pub mod response {
    pub const SUCCESSFUL: &str = "SUCCESSFUL";
    pub const INVALID: &str = "INVALID";
}

/// Basic handler that responds with a static string
pub async fn root() -> &'static str {
    "Hello, World!"
}
