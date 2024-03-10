use crate::types::{ok, AEState};
use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde_json::json;

mod offers;
mod orders;
mod products;

pub fn router<S>(state: AEState) -> Router<S> {
    Router::new()
        .route("/get/cfg", post(get_cfg))
        .nest(
            "/offers",
            Router::new()
                .route("/new", post(offers::new))
                .route("/get/:offer_id", get(offers::get))
                .route("/next", get(offers::next))
                .route("/update", post(offers::update)),
        )
        .nest(
            "/products",
            Router::new()
                .route("/new", post(products::new))
                .route("/get/:product_id", get(products::get))
                .route("/update", post(products::update))
                .route("/products_from_ids", post(products::products_from_ids))
                .route("/ship_use_stock", post(products::ship_use_stock)),
        )
        .nest(
            "/orders",
            Router::new()
                .route("/get/:oid", get(orders::get_from_order_id))
                .route("/update_or_add", post(orders::update_or_add))
                .route("/next", get(orders::next))
                .route(
                    "/:oid/update_weight/:weight/item_num/:item_num",
                    get(orders::update_weight),
                )
                .route("/set_lg_id", post(orders::set_lg_id)),
        )
        .nest(
            "/admin",
            Router::new()
                .route("/get/cfg", post(get_cfg))
                .nest(
                    "/offers",
                    Router::new()
                        .route("/show", post(offers::admin_offers_show))
                        .route("/pending/:id/:pending", get(offers::admin_offer_pending))
                        .route("/delete/:id/:tf", get(offers::admin_offer_delete))
                        .route("/tips", post(offers::admin_offer_tips))
                        .route("/pid/:id/:pid", get(offers::admin_offer_pid))
                        .route("/mid/:id/:mid", get(offers::admin_offer_mid))
                        .route(
                            "/allbetterpricechnageisok",
                            get(offers::all_better_price_chnage_is_ok),
                        ),
                )
                .nest(
                    "/products",
                    Router::new()
                        .route("/show", post(products::admin_product_show))
                        .route(
                            "/pending/:id/:pending",
                            get(products::admin_product_pending),
                        )
                        .route(
                            "/inited_weight/:id/:inited",
                            get(products::admin_product_inited_weight),
                        )
                        .route("/delete/:id/:tf", get(products::admin_product_delete))
                        .route("/tips", post(products::admin_product_tips))
                        .route("/oid/:id/:oid", get(products::admin_product_oid))
                        .route(
                            "/clear_stock_info/:id",
                            get(products::admin_product_clear_stock_info),
                        )
                        .route("/update_info", post(products::admin_product_update_info))
                        .route(
                            "/discount/:id/:discount",
                            get(products::admin_product_discount),
                        )
                        .route(
                            "/dl_discount_xslx/:default_discount",
                            get(products::admin_product_dl_discount_xslx),
                        )
                        .route("/upload_xlsx", post(products::admin_product_upload_xlsx))
                        .route("/available", get(products::admin_product_available)),
                )
                .nest(
                    "/orders",
                    Router::new().route("/show", post(orders::admin_order_show)),
                ),
        )
        .with_state(state)
}

async fn get_cfg(
    State(AEState {
        db_pool: _,
        settings,
    }): State<AEState>,
    Json(keys): Json<Vec<String>>,
) -> impl IntoResponse {
    let mut result = json!({});
    for k in keys.iter() {
        result[k] = settings[k].clone();
    }
    ok(result)
}
