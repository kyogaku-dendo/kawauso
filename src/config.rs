#[derive(Debug, Clone)]
pub struct Config {
    pub base_pdf_url: String,
    pub r2_bucket_name: String,
    pub r2_public_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Config {
            base_pdf_url: std::env::var("BASE_PDF_URL")?,
            r2_bucket_name: std::env::var("R2_BUCKET_NAME")?,
            r2_public_url: std::env::var("R2_PUBLIC_URL")?,
        })
    }
}
