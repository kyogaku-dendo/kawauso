mod config;
mod pdf_handler;
mod r2_client;
mod receipt_printer;

#[derive(Clone)]
struct AppState {
    config: std::sync::Arc<config::Config>,
    pdf_handler: std::sync::Arc<pdf_handler::PdfHandler>,
    r2_client: std::sync::Arc<r2_client::R2Client>,
    receipt_printer: std::sync::Arc<receipt_printer::ReceiptPrinter>,
}

#[derive(serde::Deserialize)]
struct PrintPdfRequest {
    count: u32,
    #[serde(rename = "paymentId")]
    payment_id: uuid::Uuid,
    #[serde(rename = "paidAt")]
    paid_at: u64,
}

#[derive(serde::Deserialize)]
struct PrintTagRequest {
    tag: String,
}

#[derive(serde::Serialize)]
struct PrintPdfResponse {
    success: bool,
    message: String,
    #[serde(rename = "paymentId")]
    payment_id: String,
    pdfs: Vec<PdfInfo>,
}

#[derive(serde::Serialize)]
struct PdfInfo {
    id: String,
    url: String,
}

#[derive(serde::Serialize)]
struct PrintTagResponse {
    success: bool,
    message: String,
}

async fn print_pdf(
    state: actix_web::web::Data<AppState>,
    req: actix_web::web::Json<PrintPdfRequest>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!(
        "\nPrint PDF request - Payment ID: {}, Count: {}, Paid at: {}",
        req.payment_id, req.count, req.paid_at
    );

    let base_pdf = state
        .pdf_handler
        .fetch_base_pdf(&state.config.base_pdf_path)
        .await
        .map_err(|e| {
            eprintln!("❌ Failed to fetch base PDF: {}", e);
            actix_web::error::ErrorInternalServerError(format!("failed to fetch base PDF: {}", e))
        })?;

    let mut pdfs = Vec::new();

    for i in 0..req.count {
        let pdf_id = uuid::Uuid::new_v4();
        println!(
            "\n[{}/{}] Processing PDF with ID: {}",
            i + 1,
            req.count,
            pdf_id
        );

        let signed_pdf = state
            .pdf_handler
            .sign_pdf(base_pdf.clone(), &pdf_id, &state.config.base_pdf_path)
            .await
            .map_err(|e| {
                eprintln!("❌ Failed to sign PDF {}: {}", pdf_id, e);
                actix_web::error::ErrorInternalServerError(format!(
                    "failed to sign PDF {}: {}",
                    pdf_id, e
                ))
            })?;

        tokio::fs::create_dir_all("signed_pdf").await.map_err(|e| {
            eprintln!("Failed to create signed_pdf directory: {}", e);
            actix_web::error::ErrorInternalServerError(format!("failed to create directory: {}", e))
        })?;

        let local_path = format!("signed_pdf/{}.pdf", pdf_id);
        println!("Saving signed PDF locally: {}", local_path);
        tokio::fs::write(&local_path, &signed_pdf)
            .await
            .map_err(|e| {
                eprintln!("❌ Failed to save signed PDF locally: {}", e);
                actix_web::error::ErrorInternalServerError(format!(
                    "failed to save signed PDF: {}",
                    e
                ))
            })?;

        let object_key = format!("signed_pdfs/{}.pdf", pdf_id);
        state
            .r2_client
            .upload_pdf(&object_key, signed_pdf)
            .await
            .map_err(|e| {
                eprintln!("❌ Failed to upload PDF to R2: {}", e);
                actix_web::error::ErrorInternalServerError(format!(
                    "failed to upload PDF to R2: {}",
                    e
                ))
            })?;

        let pdf_url = format!("{}/{}", state.config.r2_public_url, object_key);
        println!("PDF {} uploaded: {}", i + 1, pdf_url);

        pdfs.push(PdfInfo {
            id: pdf_id.to_string(),
            url: pdf_url.clone(),
        });

        // レシートを印刷
        if let Err(e) = state
            .receipt_printer
            .print_pdf_receipt(
                &pdf_url,
                &pdf_id.to_string(),
                &req.payment_id.to_string(),
                req.paid_at,
                req.count,
            )
            .await
        {
            eprintln!("⚠️ Failed to print receipt for PDF {}: {}", pdf_id, e);
            // レシートの印刷失敗はエラーを返さず続行
        }
    }

    println!("\n✓ {} QR code receipts printed", req.count);

    Ok(actix_web::HttpResponse::Ok().json(PrintPdfResponse {
        success: true,
        message: format!(
            "{} PDFs signed and uploaded. {} receipt(s) to be printed",
            req.count, req.count
        ),
        payment_id: req.payment_id.to_string(),
        pdfs,
    }))
}

async fn print_tag(
    state: actix_web::web::Data<AppState>,
    req: actix_web::web::Json<PrintTagRequest>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!("\nPrint tag request - Tag: {}", req.tag);

    // 呼び出し番号タグを印刷
    if let Err(e) = state.receipt_printer.print_tag_receipt(&req.tag).await {
        eprintln!("⚠️ Failed to print tag: {}", e);
        return Ok(
            actix_web::HttpResponse::InternalServerError().json(PrintTagResponse {
                success: false,
                message: format!("Failed to print tag: {}", e),
            }),
        );
    }

    Ok(actix_web::HttpResponse::Ok().json(PrintTagResponse {
        success: true,
        message: format!("Tag print job queued: {}", req.tag),
    }))
}

async fn cut_paper(
    state: actix_web::web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!("\nCut paper request");

    if let Err(e) = state.receipt_printer.cut_paper().await {
        eprintln!("⚠️ Failed to cut paper: {}", e);
        return Ok(
            actix_web::HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to cut paper: {}", e),
            })),
        );
    }

    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Paper cut command sent",
    })))
}

async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "kawauso-print-service"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let config = std::sync::Arc::new(config::Config::from_env().expect("failed to load env vars"));

    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let pdf_handler = std::sync::Arc::new(pdf_handler::PdfHandler::new());
    let r2_client = std::sync::Arc::new(r2_client::R2Client::new(
        s3_client,
        config.r2_bucket_name.clone(),
    ));

    let receipt_printer = std::sync::Arc::new(receipt_printer::ReceiptPrinter::new(
        config.printer_name.clone(),
    ));

    let app_state = AppState {
        config,
        pdf_handler,
        r2_client,
        receipt_printer,
    };

    let bind_address = "0.0.0.0:8080";
    println!("Starting server at: http://{}", bind_address);

    actix_web::HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        actix_web::App::new()
            .wrap(cors)
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .route("/health", actix_web::web::get().to(health_check))
            .route("/print/pdf", actix_web::web::post().to(print_pdf))
            .route("/print/tag", actix_web::web::post().to(print_tag))
            .route("/cut", actix_web::web::post().to(cut_paper))
    })
    .bind(bind_address)?
    .run()
    .await
}
