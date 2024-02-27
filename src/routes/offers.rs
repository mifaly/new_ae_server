use crate::models::{NewOffer, Offer, Product};
use crate::types::{err, ok, AEState, AeError, Res};
use anyhow::anyhow;
use axum::extract::{Json, Path, State};
use serde::Deserialize;
use serde_json::{from_str, json, Value};
use sqlx::{query, query_as, QueryBuilder};
use std::cmp::{max, min};
use time::{Duration, OffsetDateTime};

pub async fn new(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
    Json(mut no): Json<NewOffer>,
) -> Result<Res, AeError> {
    if let Some(_) = query("SELECT id FROM offers WHERE offer_id = ?1")
        .bind(no.offer_id)
        .fetch_optional(&db)
        .await?
    {
        return err("该offer_id已存在".to_string());
    }

    let offer_price_rate: f64 = settings["OFFER_PRICE_RATE"].as_f64().unwrap_or(1.5);
    //价格按倍率调整
    no.price = (no.price as f64 * offer_price_rate) as i64;
    let offer = Offer::new(&no);

    let id = query("INSERT INTO offers (product_id, sale_record, discount, sku_info_use, detail_url_use, pending, tips, created_at, updated_at, deleted_at, offer_id, title, cover, wireless_video_id, detail_video_id, model_id, sale30, sale_info, price, better_price, sku_info, detail_url, supplier, store_url, promotion_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)")
    .bind(offer.product_id)
    .bind(offer.sale_record)
    .bind(offer.discount)
    .bind(offer.sku_info_use)
    .bind(offer.detail_url_use)
    .bind(offer.pending)
    .bind(offer.tips)
    .bind(offer.created_at)
    .bind(offer.updated_at)
    .bind(offer.deleted_at)
    .bind(offer.offer_id)
    .bind(offer.title)
    .bind(offer.cover)
    .bind(offer.wireless_video_id)
    .bind(offer.detail_video_id)
    .bind(offer.model_id)
    .bind(offer.sale30)
    .bind(offer.sale_info)
    .bind(offer.price)
    .bind(offer.better_price)
    .bind(offer.sku_info)
    .bind(offer.detail_url)
    .bind(offer.supplier)
    .bind(offer.store_url)
    .bind(offer.promotion_end)
    .execute(&db).await?.last_insert_rowid();

    return ok(json!(id));
}

pub async fn get(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
    Path(oid): Path<i64>,
) -> Result<Res, AeError> {
    let offer_: Option<Offer> = query_as("SELECT * FROM offers WHERE offer_id = ?1")
        .bind(oid)
        .fetch_optional(&db)
        .await?;
    if let Some(offer) = offer_ {
        let mut res = json!(offer);
        let pd_: Option<Product> = query_as("select * from products where product_id = ?1")
            .bind(offer.product_id)
            .fetch_optional(&db)
            .await?;
        if let Some(pd) = pd_ {
            let advise_stock_num =
                (pd.sales30 as f64) * settings["SALE2STOCK"].as_f64().unwrap_or(0.67);
            let sale_info: Value = from_str(&pd.sale_info)?; //已卖出数据为基准
            let stock_info: Value = from_str(&pd.stock_info)?;
            let mut advise_stock = json!({});
            for (color, sizes) in sale_info.as_object().unwrap() {
                for (size, sold) in sizes.as_object().unwrap() {
                    advise_stock[color][size.to_uppercase()] = json!(if pd.sale_count > 0 {
                        let advise = (sold.as_f64().unwrap() * advise_stock_num) / (pd.sale_count as f64);
                        let stock = stock_info[color][size].as_f64().unwrap();
                        (advise, stock, advise - stock)
                    } else {
                        (0.0, 0.0, 0.0)
                    });
                }
            }
            res["advise_stock"] = advise_stock;
        } else {
            res["advise_stock"] = json!(());
        }
        return ok(res);
    } else {
        return err("not found".to_string());
    }
}

pub async fn next(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
) -> Result<Res, AeError> {
    let row: Option<(i64,)> = query_as("select offer_id from offers where updated_at<?1 and deleted_at is null and product_id>0 order by id asc limit 1").bind(OffsetDateTime::now_local()?.date()).fetch_optional(&db).await?;
    if let Some(r) = row {
        if let Some(offer_url_pattern) = settings["OFFER_URL_PATTERN"].as_str() {
            return ok(json!(
                offer_url_pattern.replace("{OFFER_ID}", &r.0.to_string())
            ));
        } else {
            return Err(anyhow!("no offer_url_pattern").into());
        }
    } else {
        //删除180天前废弃的offer
        query("delete from offers where deleted_at is not null and deleted_at < ?")
            .bind(OffsetDateTime::now_local()?.date() - Duration::days(180))
            .execute(&db)
            .await?;

        return err("all done".to_string());
    }
}

pub async fn update(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(no): Json<NewOffer>,
) -> Result<Res, AeError> {
    let offer_: Option<Offer> = query_as("SELECT * FROM offers WHERE offer_id = ?1")
        .bind(no.offer_id)
        .fetch_optional(&db)
        .await?;
    if let Some(old_offer) = offer_ {
        let updated_offer = old_offer.update(&no);
        let affacted_rows = query("UPDATE offers SET sale_record = ?,title = ?, cover = ?, wireless_video_id = ?, detail_video_id = ?, sale30 = ?, sale_info = ?, detail_url = ?, better_price = ?, discount = ?, pending = ?, tips = ?, sku_info = ?, supplier = ?, store_url = ?, promotion_end = ?, updated_at = ? WHERE offer_id = ?")
        .bind(&updated_offer.sale_record)
        .bind(&updated_offer.title)
        .bind(&updated_offer.cover)
        .bind(updated_offer.wireless_video_id)
        .bind(updated_offer.detail_video_id)
        .bind(updated_offer.sale30)
        .bind(&updated_offer.sale_info)
        .bind(&updated_offer.detail_url)
        .bind(updated_offer.better_price)
        .bind(updated_offer.discount)
        .bind(updated_offer.pending)
        .bind(&updated_offer.tips)
        .bind(&updated_offer.sku_info)
        .bind(&updated_offer.supplier)
        .bind(&updated_offer.store_url)
        .bind(updated_offer.promotion_end)
        .bind(updated_offer.updated_at)
        .bind(updated_offer.offer_id)
        .execute(&db).await?.rows_affected();
        if affacted_rows > 0 {
            return ok(json!(updated_offer));
        } else {
            return err("nothing changed".to_string());
        }
    } else {
        return err("not found".to_string());
    }
}

#[derive(Deserialize)]
pub struct SOReq {
    page: i64,
    per_page: i64,
    offer_id: i64,
    product_id: i64,
    model_id: String,
    supplier: String,
    pending: i64,
    deleted: bool,
}
pub async fn admin_offers_show(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(mut search): Json<SOReq>,
) -> Result<Res, AeError> {
    let mut total_query_builder = QueryBuilder::new("select count(id) from offers where 1=1 ");
    let mut offers_query_builder = QueryBuilder::new("select * from offers where 1=1 ");

    if search.offer_id > 0 {
        total_query_builder.push(" and offer_id = ");
        total_query_builder.push_bind(search.offer_id);
        offers_query_builder.push(" and offer_id = ");
        offers_query_builder.push_bind(search.offer_id);
    }
    if search.product_id > 0 {
        total_query_builder.push(" and product_id = ");
        total_query_builder.push_bind(search.product_id);
        offers_query_builder.push(" and product_id = ");
        offers_query_builder.push_bind(search.product_id);
    }
    if search.model_id.trim().len() > 0 {
        total_query_builder.push(" and model_id = ");
        total_query_builder.push_bind(&search.model_id);
        offers_query_builder.push(" and model_id = ");
        offers_query_builder.push_bind(&search.model_id);
    }
    if search.supplier.trim().len() > 0 {
        total_query_builder.push(" and supplier = ");
        total_query_builder.push_bind(&search.supplier);
        offers_query_builder.push(" and supplier = ");
        offers_query_builder.push_bind(&search.supplier);
    }
    if search.pending != 999 {
        total_query_builder.push(" and pending = ");
        total_query_builder.push_bind(search.pending);
        offers_query_builder.push(" and pending = ");
        offers_query_builder.push_bind(search.pending);
    }
    if search.deleted {
        total_query_builder.push(" and deleted_at is not null");
        offers_query_builder.push(" and deleted_at is not null");
    } else {
        total_query_builder.push(" and deleted_at is null");
        offers_query_builder.push(" and deleted_at is null");
    }

    let total: (i64,) = total_query_builder.build_query_as().fetch_one(&db).await?;
    search.per_page = if search.per_page == 0 {
        20
    } else {
        search.per_page
    };
    search.page = max(1, min(search.page, total.0 / search.per_page + 1));
    offers_query_builder.push(" order by id desc");
    offers_query_builder.push(" limit ");
    offers_query_builder.push_bind(search.per_page);
    offers_query_builder.push(" offset ");
    offers_query_builder.push_bind((search.page - 1) * search.per_page);

    let offers: Vec<Offer> = offers_query_builder.build_query_as().fetch_all(&db).await?;

    return ok(json!({
        "page": search.page,
        "per_page": search.per_page,
        "total": total.0,
        "offers": offers,
    }));
}

pub async fn admin_offer_pending(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, pending)): Path<(i64, i64)>,
) -> Result<Res, AeError> {
    let affected_rows;
    if pending == 0 {
        let _tips: Option<(String,)> = query_as("select tips from offers where id=? limit 1")
            .bind(id)
            .fetch_optional(&db)
            .await?;
        match _tips {
            None => {
                return err("not found".to_string());
            }
            Some((mut tips,)) => {
                tips = tips
                    .split(";")
                    .filter(|t| t.starts_with("!"))
                    .collect::<Vec<&str>>()
                    .join(";");
                if tips.len() > 0 {
                    tips += ";";
                }
                affected_rows = query("update offers set pending=?,tips=?,sku_info_use=sku_info,detail_url_use=detail_url where id=?").bind(pending).bind(tips).bind(id).execute(&db).await?.rows_affected();
            }
        }
    } else {
        affected_rows = query("update offers set pending=? where id=?;")
            .bind(pending)
            .bind(id)
            .execute(&db)
            .await?
            .rows_affected();
    }
    if affected_rows > 0 {
        return ok(json!(()));
    } else {
        return ok(json!("未改变任何数据"));
    }
}

pub async fn admin_offer_delete(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, tf)): Path<(i64, bool)>,
) -> Result<Res, AeError> {
    let affected_rows = if tf {
        query("update offers set deleted_at=? where id=?")
            .bind(OffsetDateTime::now_local().unwrap())
            .bind(id)
            .execute(&db)
            .await?
            .rows_affected()
    } else {
        query("update offers set deleted_at=null where id=?")
            .bind(id)
            .execute(&db)
            .await?
            .rows_affected()
    };
    if affected_rows > 0 {
        return ok(json!(()));
    } else {
        return ok(json!("未改变任何数据"));
    }
}

#[derive(Deserialize)]
pub struct OTReq {
    id: i64,
    tips: String,
}
pub async fn admin_offer_tips(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(req): Json<OTReq>,
) -> Result<Res, AeError> {
    if query("update offers set tips=? where id=?")
        .bind(req.tips)
        .bind(req.id)
        .execute(&db)
        .await?
        .rows_affected()
        > 0
    {
        return ok(json!(()));
    } else {
        return ok(json!("未改变任何数据"));
    }
}

pub async fn admin_offer_pid(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, pid)): Path<(i64, i64)>,
) -> Result<Res, AeError> {
    if query("update offers set product_id=? where id=?")
        .bind(pid)
        .bind(id)
        .execute(&db)
        .await?
        .rows_affected()
        > 0
    {
        return ok(json!(()));
    } else {
        return ok(json!("未改变任何数据"));
    }
}

pub async fn admin_offer_mid(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, mid)): Path<(i64, String)>,
) -> Result<Res, AeError> {
    if query("update offers set model_id=? where id=?")
        .bind(mid)
        .bind(id)
        .execute(&db)
        .await?
        .rows_affected()
        > 0
    {
        return ok(json!(()));
    } else {
        return ok(json!("未改变任何数据"));
    }
}

pub async fn all_better_price_chnage_is_ok(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
) -> Result<Res, AeError> {
    query("update offers set pending=0,tips=\"\" where pending=-1 and deleted_at is null and tips REGEXP '^\\d{1,2} => \\d{1,2} 折扣价变更;$';").execute(&db).await?;
    return ok(json!(()));
}
