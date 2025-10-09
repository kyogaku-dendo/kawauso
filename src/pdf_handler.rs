use anyhow::Context as _;

pub struct PdfHandler {
    client: reqwest::Client,
}

impl PdfHandler {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_base_pdf(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        // let response = self
        //     .client
        //     .get(url)
        //     .send()
        //     .await
        //     .context("failed to download PDF")?;

        // if !response.status().is_success() {
        //     anyhow::bail!("failed to download PDF: {}", response.status());
        // }

        // let bytes = response
        //     .bytes()
        //     .await
        //     .context("failed to read PDF data")?
        //     .to_vec();

        let bytes = tokio::fs::read("sample.pdf")
            .await
            .context("failed to read sample PDF")?;

        Ok(bytes)
    }

    pub async fn sign_pdf(
        &self,
        pdf_data: Vec<u8>,
        pdf_id: &uuid::Uuid,
    ) -> anyhow::Result<Vec<u8>> {
        let cert =
            std::fs::read_to_string("cert/cert.crt").context("failed to read certificate")?;
        let private_key_data =
            std::fs::read_to_string("cert/key.pem").context("failed to read private key")?;

        let x509_cert = x509_certificate::CapturedX509Certificate::from_pem(cert)
            .context("failed to parse certificate")?;
        let private_key =
            x509_certificate::InMemorySigningKeyPair::from_pkcs8_pem(&private_key_data)
                .context("failed to parse private key")?;

        let signer = cryptographic_message_syntax::SignerBuilder::new(&private_key, x509_cert);

        let mut pdf_doc =
            pdf_signing::PDFSigningDocument::read_from(&*pdf_data, format!("base_{}.pdf", pdf_id))
                .map_err(|e| anyhow::anyhow!("failed to read PDF: {:?}", e))?;

        let user_signature_info = vec![pdf_signing::UserSignatureInfo {
            user_id: pdf_id.to_string(),
            user_name: "Kyogaku no dendo".to_owned(),
            user_email: "kawauso@kyogaku.example.com".to_owned(),
            user_signature: vec![],
            user_signing_keys: signer,
        }];

        let signed_pdf = pdf_doc
            .sign_document(user_signature_info)
            .map_err(|e| anyhow::anyhow!("failed to sign PDF: {:?}", e))?;

        Ok(signed_pdf)
    }
}
