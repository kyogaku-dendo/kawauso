mod config;
mod pdf_handler;
mod r2_client;

#[derive(Clone)]
struct AppState {
    config: std::sync::Arc<config::Config>,
    pdf_handler: std::sync::Arc<pdf_handler::PdfHandler>,
    r2_client: std::sync::Arc<r2_client::R2Client>,
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
    #[serde(rename = "orderId")]
    order_id: uuid::Uuid,
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
        println!("\n[{}/{}] Processing PDF with ID: {}", i + 1, req.count, pdf_id);

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

        let local_path = format!("signed_{}.pdf", pdf_id);
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
            url: pdf_url,
        });
    }

    println!(
        "\nTODO: Print {} QR code receipts (one per PDF)",
        req.count
    );

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
    _state: actix_web::web::Data<AppState>,
    req: actix_web::web::Json<PrintTagRequest>,
) -> actix_web::Result<actix_web::HttpResponse> {
    println!(
        "\nPrint tag request - Order ID: {}, Tag: {}",
        req.order_id, req.tag
    );

    println!("TODO: Tag print");

    Ok(actix_web::HttpResponse::Ok().json(PrintTagResponse {
        success: true,
        message: format!("Tag print job queued: {} (order: {})", req.tag, req.order_id),
    }))
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

    let app_state = AppState {
        config,
        pdf_handler,
        r2_client,
    };

    let bind_address = "127.0.0.1:8080";
    println!("Starting server at: http://{}", bind_address);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .route("/health", actix_web::web::get().to(health_check))
            .route("/print/pdf", actix_web::web::post().to(print_pdf))
            .route("/print/tag", actix_web::web::post().to(print_tag))
    })
    .bind(bind_address)?
    .run()
    .await
}
