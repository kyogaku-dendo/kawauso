use anyhow::Context as _;

pub struct PdfHandler;

impl PdfHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_base_pdf(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let bytes = tokio::fs::read(path)
            .await
            .context(format!("failed to read PDF from path: {}", path))?;

        Ok(bytes)
    }

    fn add_signature_field_to_pdf(&self, pdf_data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let input_temp_path = format!("/tmp/input_pdf_{}.pdf", uuid::Uuid::new_v4());
        let output_temp_path = format!("/tmp/output_pdf_{}.pdf", uuid::Uuid::new_v4());

        std::fs::write(&input_temp_path, pdf_data).context("Failed to write input temp PDF")?;

        let result = std::process::Command::new("python3")
            .arg("add_sigfield_pikepdf.py")
            .arg(&input_temp_path)
            .arg(&output_temp_path)
            .output()
            .context("Failed to run pikepdf script")?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            eprintln!("pikepdf script failed:");
            eprintln!("stdout: {}", stdout);
            eprintln!("stderr: {}", stderr);

            let _ = std::fs::remove_file(&input_temp_path);
            let _ = std::fs::remove_file(&output_temp_path);

            return Err(anyhow::anyhow!("Failed to add signature field: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&result.stdout);
        println!("   {}", stdout.trim());

        let output = std::fs::read(&output_temp_path).context("Failed to read output PDF")?;
        println!("   Read output PDF: {} bytes", output.len());

        let _ = std::fs::remove_file(&input_temp_path);
        let _ = std::fs::remove_file(&output_temp_path);

        Ok(output)
    }

    fn has_signature_fields(&self, pdf_data: &[u8], pdf_name: &str) -> anyhow::Result<bool> {
        let pdf_doc = pdf_signing::PDFSigningDocument::read_from(pdf_data, pdf_name.to_string())
            .map_err(|_| anyhow::anyhow!("Failed to read PDF"))?;

        let doc = pdf_doc.get_prev_document_ref();

        if let Ok(catalog) = doc.catalog()
            && let Ok(acro_form) = catalog.get(b"AcroForm")
            && let Ok(acro_form_dict) = acro_form.as_dict()
            && let Ok(fields) = acro_form_dict.get(b"Fields")
            && let Ok(fields_array) = fields.as_array()
        {
            // 署名フィールドが存在するか確認
            return Ok(!fields_array.is_empty());
        }

        Ok(false)
    }

    pub async fn sign_pdf(
        &self,
        pdf_data: Vec<u8>,
        pdf_id: &uuid::Uuid,
        base_pdf_path: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let pdf_name = std::path::Path::new(base_pdf_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("base.pdf");
        println!("🖋  Signing PDF: {}", pdf_name);

        // 署名フィールドがない場合は追加
        let has_sig = self.has_signature_fields(&pdf_data, pdf_name)?;
        println!("Signature field exists: {}", has_sig);

        let pdf_with_field = if !has_sig {
            println!("📝 Adding signature field...");
            self.add_signature_field_to_pdf(&pdf_data)?
        } else {
            println!("Signature field already exists");
            pdf_data
        };

        // 一時ファイルに保存
        let input_temp_path = format!("/tmp/input_{}.pdf", pdf_id);
        let output_temp_path = format!("/tmp/signed_{}.pdf", pdf_id);

        std::fs::write(&input_temp_path, &pdf_with_field)
            .context("Failed to write input temp PDF")?;

        let result = std::process::Command::new("python3")
            .arg("sign_pdf_pyhanko.py")
            .arg(&input_temp_path)
            .arg(&output_temp_path)
            .arg("cert/cert.crt")
            .arg("cert/key.pem")
            .arg("KyogakuDendoSignature")
            .arg(pdf_id.to_string()) // UUIDをreasonフィールドに記録
            .output()
            .context("Failed to run pyhanko signing script")?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            eprintln!("❌ pyhanko signing failed:");
            eprintln!("stdout: {}", stdout);
            eprintln!("stderr: {}", stderr);

            // クリーンアップ
            let _ = std::fs::remove_file(&input_temp_path);
            let _ = std::fs::remove_file(&output_temp_path);

            return Err(anyhow::anyhow!(
                "Failed to sign PDF with pyhanko: {}",
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&result.stdout);
        println!("   {}", stdout.trim());
        println!("✓ Successfully signed PDF");

        let signed_pdf = std::fs::read(&output_temp_path).context("Failed to read signed PDF")?;

        println!("🔍 Verifying signature...");
        let verify_result = std::process::Command::new("pdfsig")
            .arg(&output_temp_path)
            .output();

        match verify_result {
            Ok(verify_output) => {
                let stdout = String::from_utf8_lossy(&verify_output.stdout);
                println!("pdfsig output:\n{}", stdout);
            }
            Err(e) => {
                println!("⚠️  Failed to run pdfsig: {}", e);
            }
        }

        let _ = std::fs::remove_file(&input_temp_path);
        let _ = std::fs::remove_file(&output_temp_path);

        Ok(signed_pdf)
    }
}

impl Default for PdfHandler {
    fn default() -> Self {
        Self::new()
    }
}
