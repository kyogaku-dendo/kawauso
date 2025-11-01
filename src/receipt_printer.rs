use anyhow::Context as _;
use chrono::TimeZone as _;

pub struct ReceiptPrinter {
    printer_name: String,
    receipts_dir: std::path::PathBuf,
}

impl ReceiptPrinter {
    pub fn new(printer_name: String) -> Self {
        Self {
            printer_name,
            receipts_dir: std::path::PathBuf::from("receipts"),
        }
    }

    /// QRコード付きレシートを印刷
    pub async fn print_pdf_receipt(
        &self,
        pdf_url: &str,
        pdf_id: &str,
        payment_id: &str,
        paid_at: u64,
        count: u32,
    ) -> anyhow::Result<()> {
        // receiptsディレクトリを作成
        tokio::fs::create_dir_all(&self.receipts_dir)
            .await
            .context("Failed to create receipts directory")?;

        // ESC/POSバイナリファイルのパス
        let receipt_filename = format!("receipt_{}.bin", pdf_id);
        let receipt_path = self.receipts_dir.join(&receipt_filename);

        // ESC/POSコマンドを生成
        self.generate_receipt(&receipt_path, pdf_url, pdf_id, payment_id, paid_at, count)?;

        // lprコマンドで印刷ジョブをキューイング
        self.send_to_printer(&receipt_path).await?;

        println!("✓ Receipt printed: {}", receipt_filename);

        Ok(())
    }

    /// ESC/POSコマンドを生成
    fn generate_receipt(
        &self,
        path: &std::path::PathBuf,
        pdf_url: &str,
        pdf_id: &str,
        payment_id: &str,
        paid_at: u64,
        count: u32,
    ) -> anyhow::Result<()> {
        std::fs::File::create(path).context("Failed to create receipt file")?;

        let driver =
            escpos::driver::FileDriver::open(path).context("Failed to open file driver")?;

        let dt_utc = chrono::DateTime::from_timestamp(paid_at as i64, 0)
            .unwrap_or_else(|| chrono::Utc.timestamp_opt(0, 0).unwrap());
        let dt_jst = dt_utc + chrono::Duration::hours(9);
        let paid_at_display = dt_jst.format("%Y/%m/%d %H:%M:%S").to_string();

        // プリンターを初期化
        escpos::printer::Printer::new(
            driver,
            Default::default(),
            Some(escpos::printer_options::PrinterOptions::default()),
        )
        .init()
        .context("Failed to init printer")?
        .justify(escpos::utils::JustifyMode::CENTER)
        .context("Failed to set justify")?
        .size(1, 1)?
        .bit_image_option(
            "./img/npo_top.png",
            escpos::utils::BitImageOption::new(
                Some(400),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/book_receipt.png",
            escpos::utils::BitImageOption::new(
                Some(600),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .qrcode(pdf_url)
        .context("Failed to write QR code")?
        .bit_image_option(
            "./img/qr-instruction.png",
            escpos::utils::BitImageOption::new(
                Some(600),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/white.png",
            escpos::utils::BitImageOption::new(
                Some(400),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .writeln(&format!("PDF ID: {}", &pdf_id[..8]))
        .context("Failed to write PDF ID")?
        .writeln(&format!("Payment ID: {}", &payment_id[..8]))
        .context("Failed to write payment ID")?
        .bit_image_option(
            "./img/white.png",
            escpos::utils::BitImageOption::new(
                Some(400),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/date.png",
            escpos::utils::BitImageOption::new(
                Some(200),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .writeln(&paid_at_display)
        .context("Failed to write paid at")?
        .writeln("Thank you!")
        .context("Failed to write footer")?
        .feed()
        .context("Failed to feed")?
        .print_cut()
        .context("Failed to cut")?;

        Ok(())
    }

    async fn send_to_printer(&self, receipt_path: &std::path::PathBuf) -> anyhow::Result<()> {
        let output = tokio::process::Command::new("lpr")
            .arg("-P")
            .arg(&self.printer_name)
            .arg("-l") // RAWモード
            .arg(receipt_path)
            .output()
            .await
            .context("Failed to execute lpr command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("lpr command failed: {}", stderr));
        }

        Ok(())
    }

    /// 呼び出し番号タグを印刷
    pub async fn print_tag_receipt(&self, tag: &str) -> anyhow::Result<()> {
        // receiptsディレクトリを作成
        tokio::fs::create_dir_all(&self.receipts_dir)
            .await
            .context("Failed to create receipts directory")?;

        // ESC/POSバイナリファイルのパス（タグ名を使用）
        let receipt_filename = format!("tag_{}.bin", tag);
        let receipt_path = self.receipts_dir.join(&receipt_filename);

        // ESC/POSコマンドを生成
        self.generate_tag(&receipt_path, tag)?;

        // lprコマンドで印刷ジョブをキューイング
        self.send_to_printer(&receipt_path).await?;

        println!("✓ Tag printed: {}", receipt_filename);

        Ok(())
    }

    /// 呼び出し番号タグのESC/POSコマンドを生成
    fn generate_tag(&self, path: &std::path::PathBuf, tag: &str) -> anyhow::Result<()> {
        // ファイルを作成
        std::fs::File::create(path).context("Failed to create tag file")?;

        // ESC/POSドライバを初期化
        let driver =
            escpos::driver::FileDriver::open(path).context("Failed to open file driver")?;

        // プリンターを初期化
        escpos::printer::Printer::new(
            driver,
            Default::default(),
            Some(escpos::printer_options::PrinterOptions::default()),
        )
        .init()
        .context("Failed to init printer")?
        .justify(escpos::utils::JustifyMode::CENTER)
        .context("Failed to set justify")?
        .bit_image_option(
            "./img/npo_top.png",
            escpos::utils::BitImageOption::new(
                Some(400),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/white.png",
            escpos::utils::BitImageOption::new(
                Some(640),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/callnumber.png",
            escpos::utils::BitImageOption::new(
                Some(320),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/white.png",
            escpos::utils::BitImageOption::new(
                Some(480),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .size(3, 4)?
        .writeln(&format!("[ {} ]", tag))
        .context("Failed to write tag")?
        .bit_image_option(
            "./img/white.png",
            escpos::utils::BitImageOption::new(
                Some(480),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/orders.png",
            escpos::utils::BitImageOption::new(
                Some(320),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/signage.png",
            escpos::utils::BitImageOption::new(
                Some(600),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .bit_image_option(
            "./img/drink.png",
            escpos::utils::BitImageOption::new(
                Some(600),
                None,
                escpos::utils::BitImageSize::Normal,
            )?,
        )?
        .feed()
        .context("Failed to feed")?
        .print_cut()
        .context("Failed to cut")?;

        Ok(())
    }

    // 紙詰まり時などに紙を切る
    pub async fn cut_paper(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.receipts_dir)
            .await
            .context("Failed to create receipts directory")?;

        let receipt_filename = "cut.bin";
        let receipt_path = self.receipts_dir.join(receipt_filename);

        self.generate_cut(&receipt_path)?;

        self.send_to_printer(&receipt_path).await?;

        println!("✓ Paper cut command sent: {}", receipt_filename);

        Ok(())
    }

    fn generate_cut(&self, path: &std::path::PathBuf) -> anyhow::Result<()> {
        std::fs::File::create(path).context("Failed to create cut file")?;
        let driver =
            escpos::driver::FileDriver::open(path).context("Failed to open file driver")?;

        escpos::printer::Printer::new(
            driver,
            Default::default(),
            Some(escpos::printer_options::PrinterOptions::default()),
        )
        .init()
        .context("Failed to init printer")?
        .writeln("--- Cut Paper ---")
        .context("Failed to write header")?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .writeln("")
        .context("Failed to write newline")?
        .print_cut()
        .context("Failed to cut")?;

        Ok(())
    }
}
