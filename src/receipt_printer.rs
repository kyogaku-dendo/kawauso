use anyhow::{Context, Result};
use escpos::{
    driver::FileDriver, printer::Printer, printer_options::PrinterOptions, utils::JustifyMode,
};
use std::fs::File;
use std::path::PathBuf;

pub struct ReceiptPrinter {
    printer_name: String,
    receipts_dir: PathBuf,
}

impl ReceiptPrinter {
    pub fn new(printer_name: String) -> Self {
        Self {
            printer_name,
            receipts_dir: PathBuf::from("receipts"),
        }
    }

    /// QRコード付きレシートを印刷
    pub async fn print_pdf_receipt(
        &self,
        pdf_url: &str,
        pdf_id: &str,
        payment_id: &str,
        count: u32,
    ) -> Result<()> {
        // receiptsディレクトリを作成
        tokio::fs::create_dir_all(&self.receipts_dir)
            .await
            .context("Failed to create receipts directory")?;

        // ESC/POSバイナリファイルのパス
        let receipt_filename = format!("receipt_{}.bin", pdf_id);
        let receipt_path = self.receipts_dir.join(&receipt_filename);

        // ESC/POSコマンドを生成
        self.generate_receipt(&receipt_path, pdf_url, pdf_id, payment_id, count)?;

        // lprコマンドで印刷ジョブをキューイング
        self.send_to_printer(&receipt_path).await?;

        println!("✓ Receipt printed: {}", receipt_filename);

        Ok(())
    }

    /// ESC/POSコマンドを生成
    fn generate_receipt(
        &self,
        path: &PathBuf,
        pdf_url: &str,
        pdf_id: &str,
        payment_id: &str,
        count: u32,
    ) -> Result<()> {
        // ファイルを作成
        File::create(path).context("Failed to create receipt file")?;

        // ESC/POSドライバを初期化
        let driver = FileDriver::open(path).context("Failed to open file driver")?;

        // プリンターを初期化
        Printer::new(driver, Default::default(), Some(PrinterOptions::default()))
            .init()
            .context("Failed to init printer")?
            .justify(JustifyMode::CENTER)
            .context("Failed to set justify")?
            .writeln("kyogaku-dendo")
            .context("Failed to write header")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln(&format!("フランクフルト x {}", count))
            .context("Failed to write item")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln("同人誌PDF")
            .context("Failed to write description")?
            .writeln("下記のQRコードをスキャン")
            .context("Failed to write instruction")?
            .writeln("")
            .context("Failed to write newline")?
            .qrcode(pdf_url)
            .context("Failed to write QR code")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln(&format!("PDF ID: {}", &pdf_id[..8]))
            .context("Failed to write PDF ID")?
            .writeln(&format!("Payment: {}", &payment_id[..8]))
            .context("Failed to write payment ID")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln("ありがとうございました")
            .context("Failed to write footer")?
            .feed()
            .context("Failed to feed")?
            .print_cut()
            .context("Failed to cut")?;

        Ok(())
    }

    /// lprコマンドで印刷ジョブを送信
    async fn send_to_printer(&self, receipt_path: &PathBuf) -> Result<()> {
        println!("📤 Sending to printer: {}", self.printer_name);

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
    pub async fn print_tag_receipt(&self, tag: &str, order_id: &str) -> Result<()> {
        // receiptsディレクトリを作成
        tokio::fs::create_dir_all(&self.receipts_dir)
            .await
            .context("Failed to create receipts directory")?;

        // ESC/POSバイナリファイルのパス
        let receipt_filename = format!("tag_{}.bin", order_id);
        let receipt_path = self.receipts_dir.join(&receipt_filename);

        // ESC/POSコマンドを生成
        self.generate_tag(&receipt_path, tag, order_id)?;

        // lprコマンドで印刷ジョブをキューイング
        self.send_to_printer(&receipt_path).await?;

        println!("✓ Tag printed: {}", receipt_filename);

        Ok(())
    }

    /// 呼び出し番号タグのESC/POSコマンドを生成
    fn generate_tag(&self, path: &PathBuf, tag: &str, order_id: &str) -> Result<()> {
        // ファイルを作成
        File::create(path).context("Failed to create tag file")?;

        // ESC/POSドライバを初期化
        let driver = FileDriver::open(path).context("Failed to open file driver")?;

        // プリンターを初期化
        Printer::new(driver, Default::default(), Some(PrinterOptions::default()))
            .init()
            .context("Failed to init printer")?
            .justify(JustifyMode::CENTER)
            .context("Failed to set justify")?
            .writeln("kyogaku-dendo")
            .context("Failed to write header")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln("お呼び出し番号")
            .context("Failed to write title")?
            .writeln("")
            .context("Failed to write newline")?
            // TODO: 大きなフォントで番号を表示（後で調整）
            .writeln(&format!("[ {} ]", tag))
            .context("Failed to write tag")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln(&format!("Order: {}", &order_id[..8]))
            .context("Failed to write order ID")?
            .writeln("")
            .context("Failed to write newline")?
            .writeln("しばらくお待ちください")
            .context("Failed to write footer")?
            .feed()
            .context("Failed to feed")?
            .print_cut()
            .context("Failed to cut")?;

        Ok(())
    }
}
