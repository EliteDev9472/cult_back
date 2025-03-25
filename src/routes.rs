// use actix_web::{web, HttpResponse, get};
// use crate::handlers;

// pub fn ido_config(cfg: &mut web::ServiceConfig) {
//     cfg.service(
//         web::scope("/api/idos")
//             .route("", web::post().to(handlers::create_ido))
//             .route("", web::get().to(handlers::get_ido))
//             .route("/all", web::get().to(handlers::get_all_idos))
//             .route("/{contract_address}", web::get().to(handlers::get_ido_by_address))
//             .route("/{id}", web::patch().to(handlers::update_ido))
//             .route("/{id}", web::delete().to(handlers::delete_ido)),
//     );
// }

// #[get("/api/health")]
// pub async fn health_check() -> HttpResponse {
//     HttpResponse::Ok().json(serde_json::json!({
//         "status": "Backend is working!"
//     }))
// }