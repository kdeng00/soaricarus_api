pub mod maze {
    pub fn get_config() -> labyrinth::config::Config {
        let s3_endpoint_url = sienvy::environment::get_env("S3_ENDPOINT_URL");
        let bucket_name = sienvy::environment::get_env("S3_BUCKET_NAME");
        let region = sienvy::environment::get_env("GARAGE_S3_REGION");
        let access_key_id = sienvy::environment::get_env("GARAGE_DEFAULT_ACCESS_KEY");
        let secret_key = sienvy::environment::get_env("GARAGE_DEFAULT_SECRET_KEY");

        labyrinth::config::Config {
            url: s3_endpoint_url.value,
            bucket: bucket_name.value,
            region: region.value,
            access_key_id: access_key_id.value,
            secret_key: secret_key.value,
        }
    }
}
