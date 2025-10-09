mod config;
mod pdf_handler;
mod r2_client;

#[derive(Clone)]
struct AppState {
    config: std::sync::Arc<config::Config>,
    pdf_handler: std::sync::Arc<pdf_handler::PdfHandler>,
    r2_client: std::sync::Arc<r2_client::R2Client>,
}

#[derive(serde::Serialize)]
struct SignedPdfResponse {
    id: String,
    url: String,
}

async fn sign_pdf(
    state: actix_web::web::Data<AppState>,
) -> actix_web::Result<actix_web::HttpResponse> {
    let pdf_id = uuid::Uuid::new_v4();

    let base_pdf = state
        .pdf_handler
        .fetch_base_pdf(&state.config.base_pdf_url)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("failed to fetch base PDF: {}", e))
        })?;

    let signed_pdf = state
        .pdf_handler
        .sign_pdf(base_pdf, &pdf_id)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("failed to sign PDF: {}", e))
        })?;

    let local_path = format!("signed_{}.pdf", pdf_id);
    tokio::fs::write(&local_path, &signed_pdf)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("failed to save signed PDF: {}", e))
        })?;

    let object_key = format!("signed_pdfs/{}.pdf", pdf_id);
    // state
    //     .r2_client
    //     .upload_pdf(&object_key, signed_pdf)
    //     .await
    //     .map_err(|e| {
    //         actix_web::error::ErrorInternalServerError(format!("failed to upload PDF to R2: {}", e))
    //     })?;

    let pdf_url = format!("{}/{}", state.config.r2_public_url, object_key);

    Ok(actix_web::HttpResponse::Ok().json(SignedPdfResponse {
        id: pdf_id.to_string(),
        url: pdf_url,
    }))
}

async fn health_check() -> actix_web::Result<actix_web::HttpResponse> {
    Ok(actix_web::HttpResponse::Ok().body("Hi!"))
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
            .route("/sign-pdf", actix_web::web::post().to(sign_pdf))
    })
    .bind(bind_address)?
    .run()
    .await
}
