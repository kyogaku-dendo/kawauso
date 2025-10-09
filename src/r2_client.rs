use anyhow::Context as _;

pub struct R2Client {
    client: aws_sdk_s3::Client,
    bucket_name: String,
}

impl R2Client {
    pub fn new(client: aws_sdk_s3::Client, bucket_name: String) -> Self {
        Self {
            client,
            bucket_name,
        }
    }

    pub async fn upload_pdf(&self, object_key: &str, pdf_data: Vec<u8>) -> anyhow::Result<()> {
        let body = aws_sdk_s3::primitives::ByteStream::from(pdf_data);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(object_key)
            .body(body)
            .content_type("application/pdf")
            .send()
            .await
            .context("failed to upload PDF to R2")?;

        Ok(())
    }
}
