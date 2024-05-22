use crate::models::{NewProduct, Offer, Product};
use crate::types::{err, ok, AEState, AeError, Res};
use axum::{
    body::Body,
    extract::Multipart,
    extract::{Json, Path, State},
    http::StatusCode,
    response::Response,
};
use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use sqlx::{query, query_as, FromRow, QueryBuilder, Row};
use std::{
    cmp::{max, min},
    collections::HashMap,
    fs,
    path::PathBuf,
};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use tracing::{debug, error};

pub async fn new(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(np): Json<NewProduct>,
) -> Result<Res, AeError> {
    if let Some(_) = query("SELECT id FROM products WHERE product_id = ?1")
        .bind(np.product_id)
        .fetch_optional(&db)
        .await?
    {
        return err("该product_id已存在".to_string());
    }

    let product = Product::new(&np);

    let id = query("INSERT INTO products (uv30,sales30,sale_record,offer_id,discount,stock_count,sale_count,sale_info,sale_weight,weight_cal_count,weight,inited_weight,pending,tips,created_at,updated_at,deleted_at,product_id,title,cover,price,stock_info,model_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)")
    .bind(product.uv30)
    .bind(product.sales30)
    .bind(product.sale_record)
    .bind(product.offer_id)
    .bind(product.discount)
    .bind(product.stock_count)
    .bind(product.sale_count)
    .bind(product.sale_info)
    .bind(product.sale_weight)
    .bind(product.weight_cal_count)
    .bind(product.weight)
    .bind(product.inited_weight)
    .bind(product.pending)
    .bind(product.tips)
    .bind(product.created_at)
    .bind(product.updated_at)
    .bind(product.deleted_at)
    .bind(product.product_id)
    .bind(product.title)
    .bind(product.cover)
    .bind(product.price)
    .bind(product.stock_info)
    .bind(product.model_id)
    .execute(&db).await?.last_insert_rowid();

    return ok(json!(id));
}

pub async fn get(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path(pid): Path<i64>,
) -> Result<Res, AeError> {
    let product_: Option<Product> = query_as("SELECT * FROM products WHERE product_id = ?1")
        .bind(pid)
        .fetch_optional(&db)
        .await?;
    if let Some(product) = product_ {
        return ok(json!(product));
    } else {
        return err("not found".to_string());
    }
}

pub async fn update(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(np): Json<NewProduct>,
) -> Result<Res, AeError> {
    let product_: Option<Product> = query_as("SELECT * FROM products WHERE product_id = ?1")
        .bind(np.product_id)
        .fetch_optional(&db)
        .await?;
    if let Some(old_product) = product_ {
        let updated_product = old_product.update(&np);
        let affacted_rows = query("update products set title=?, cover=?, price=?, stock_info=?, sale_info=?, model_id=?, updated_at=? where product_id = ?")
        .bind(&updated_product.title)
        .bind(&updated_product.cover)
        .bind(updated_product.price)
        .bind(&updated_product.stock_info)
        .bind(&updated_product.sale_info)
        .bind(&updated_product.model_id)
        .bind(updated_product.updated_at)
        .bind(updated_product.product_id)
        .execute(&db).await?.rows_affected();

        if affacted_rows > 0 {
            return ok(json!(updated_product));
        } else {
            return err("nothing changed".to_string());
        }
    } else {
        return err("not found".to_string());
    }
}

pub async fn products_from_ids(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(product_ids): Json<Vec<i64>>,
) -> Result<Res, AeError> {
    if product_ids.len() <= 0 || product_ids.len() > 100 {
        return ok(json!(()));
        //return err("product_ids length must between 1 and 100".to_string());
    }

    #[derive(Serialize)]
    struct Of {
        offer_id: i64,
        better_price: i64,
        model_id: String,
        supplier: String,
        del: &'static str,
        sku_props_colors: String,
    }
    let mut ofs: HashMap<i64, Vec<Of>> = HashMap::new();
    let query_str = format!(
        "SELECT * FROM offers WHERE product_id in ({})",
        vec!["?"; product_ids.len()].join(",")
    );
    let mut query_ = query(&query_str);
    for pid in &product_ids {
        query_ = query_.bind(pid);
    }
    query_
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|row| match Offer::from_row(&row) {
            Ok(offer) => {
                let of = Of {
                    offer_id: offer.offer_id,
                    better_price: offer.better_price,
                    model_id: offer.model_id,
                    supplier: offer.supplier,
                    del: if let Some(_) = offer.deleted_at {
                        "❌"
                    } else {
                        ""
                    },
                    sku_props_colors: (serde_json::from_str::<Value>(&offer.sku_info_use).unwrap())
                        ["skuProps"][0]["value"]
                        .to_string(),
                };
                if ofs.contains_key(&offer.product_id) {
                    ofs.get_mut(&offer.product_id).unwrap().push(of);
                } else {
                    ofs.insert(offer.product_id, vec![of]);
                }
            }
            Err(e) => {
                error!("parse offer error: {:?}", e);
            }
        })
        .count();
    let query_str = format!(
        "SELECT * FROM products WHERE product_id in ({})",
        vec!["?"; product_ids.len()].join(",")
    );
    let mut query_ = query(&query_str);
    for pid in &product_ids {
        query_ = query_.bind(pid);
    }
    let products: HashMap<i64, (Product, Vec<Of>)> = query_
        .fetch_all(&db)
        .await?
        .into_iter()
        .filter_map(|row| match Product::from_row(&row) {
            Ok(product) => {
                let pid = product.product_id; //解决product move后使用的问题
                Some((
                    product.product_id,
                    (product, ofs.remove(&pid).unwrap_or(vec![])),
                ))
            }
            Err(e) => {
                error!("parse product error: {:?}", e);
                None
            }
        })
        .collect();

    return ok(json!(products));
}

#[derive(Deserialize)]
pub struct UseStock {
    id: i64,
    sku: [String; 2],
    quantity: i64,
    order_id: i64,
    stk: String,
}
pub async fn ship_use_stock(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(mut want): Json<UseStock>,
) -> Result<Res, AeError> {
    let product_: Option<Product> = query_as("SELECT * FROM products WHERE id = ?1")
        .bind(want.id)
        .fetch_optional(&db)
        .await?;
    if let Some(product) = product_ {
        //避免大小写造成的不匹配
        want.sku[0] = want.sku[0].to_uppercase();
        want.sku[1] = want.sku[1].to_uppercase();

        let mut stock_info: Value = serde_json::from_str(&product.stock_info.to_uppercase())?;
        if let Some(quantity_in_stock) = stock_info[&want.sku[0]][&want.sku[1]].as_i64() {
            if quantity_in_stock >= want.quantity {
                stock_info[&want.sku[0]][&want.sku[1]] = json!(quantity_in_stock - want.quantity);
                let stock_count = product.stock_count - want.quantity;
                let stock_info = serde_json::to_string_pretty(&stock_info)?;

                // 开始事务
                let mut db_trans = db.begin().await?;

                let affacted_rows = query("update orders set used_stock=? where order_id=?")
                    .bind(want.stk)
                    .bind(want.order_id)
                    .execute(&mut *db_trans)
                    .await?
                    .rows_affected();
                if affacted_rows == 0 {
                    db_trans.rollback().await?;
                    error!("orders未更新, 请手动检查");
                    return err("orders未更新, 请手动检查".to_string());
                }

                let affacted_rows =
                    query("update products set stock_count=?,stock_info=?,updated_at=? where id=?")
                        .bind(stock_count)
                        .bind(stock_info)
                        .bind(OffsetDateTime::now_local()?)
                        .bind(want.id)
                        .execute(&mut *db_trans)
                        .await?
                        .rows_affected();
                if affacted_rows == 0 {
                    db_trans.rollback().await?;
                    error!("products未更新, 请手动检查");
                    return err("products未更新, 请手动检查".to_string());
                }

                // 提交事务
                db_trans.commit().await?;

                return ok(json!(()));
            } else {
                return err("库存不足".to_string());
            }
        } else {
            return err("没有该sku库存记录".to_string());
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
    inited_weight: i64,
    pending: i64,
    deleted: bool,
}
pub async fn admin_product_show(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(mut search): Json<SOReq>,
) -> Result<Res, AeError> {
    let mut total_query_builder = QueryBuilder::new("select count(id) from products where 1=1 ");
    let mut products_query_builder = QueryBuilder::new("select * from products where 1=1 ");

    if search.offer_id > 0 {
        total_query_builder.push(" and offer_id = ");
        total_query_builder.push_bind(search.offer_id);
        products_query_builder.push(" and offer_id = ");
        products_query_builder.push_bind(search.offer_id);
    }
    if search.product_id > 0 {
        total_query_builder.push(" and product_id = ");
        total_query_builder.push_bind(search.product_id);
        products_query_builder.push(" and product_id = ");
        products_query_builder.push_bind(search.product_id);
    }
    if search.inited_weight > -1 {
        total_query_builder.push(" and inited_weight = ");
        total_query_builder.push_bind(search.inited_weight);
        products_query_builder.push(" and inited_weight = ");
        products_query_builder.push_bind(search.inited_weight);
    }
    if search.pending != 999 {
        total_query_builder.push(" and pending = ");
        total_query_builder.push_bind(search.pending);
        products_query_builder.push(" and pending = ");
        products_query_builder.push_bind(search.pending);
    }
    if search.deleted {
        total_query_builder.push(" and deleted_at is not null");
        products_query_builder.push(" and deleted_at is not null");
    } else {
        total_query_builder.push(" and deleted_at is null");
        products_query_builder.push(" and deleted_at is null");
    }

    let total: (i64,) = total_query_builder.build_query_as().fetch_one(&db).await?;
    search.per_page = if search.per_page == 0 {
        20
    } else {
        search.per_page
    };
    search.page = max(1, min(search.page, total.0 / search.per_page + 1));
    products_query_builder.push(" order by id desc");
    products_query_builder.push(" limit ");
    products_query_builder.push_bind(search.per_page);
    products_query_builder.push(" offset ");
    products_query_builder.push_bind((search.page - 1) * search.per_page);

    let products: Vec<Product> = products_query_builder
        .build_query_as()
        .fetch_all(&db)
        .await?;

    return ok(json!({
        "page": search.page,
        "per_page": search.per_page,
        "total": total.0,
        "products": products,
    }));
}

pub async fn admin_product_pending(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, pending)): Path<(i64, i64)>,
) -> Result<Res, AeError> {
    if query("update products set pending=? where id=?")
        .bind(pending)
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

pub async fn admin_product_inited_weight(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, inited)): Path<(i64, bool)>,
) -> Result<Res, AeError> {
    let inited: i64 = if inited { 1 } else { 0 };
    if query("update products set inited_weight=? where id=?")
        .bind(inited)
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

pub async fn admin_product_delete(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, tf)): Path<(i64, bool)>,
) -> Result<Res, AeError> {
    let affected_rows = if tf {
        query("update products set deleted_at=? where id=?")
            .bind(OffsetDateTime::now_local().unwrap())
            .bind(id)
            .execute(&db)
            .await?
            .rows_affected()
    } else {
        query("update products set deleted_at=null where id=?")
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
pub struct PTReq {
    id: i64,
    tips: String,
}
pub async fn admin_product_tips(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(req): Json<PTReq>,
) -> Result<Res, AeError> {
    if query("update products set tips=? where id=?")
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

pub async fn admin_product_oid(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, oid)): Path<(i64, i64)>,
) -> Result<Res, AeError> {
    if query("update products set offer_id=? where id=?")
        .bind(oid)
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

pub async fn admin_product_clear_stock_info(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id,)): Path<(i64,)>,
) -> Result<Res, AeError> {
    if query("update products set stock_info=\"\" where id=?")
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

#[derive(Deserialize)]
pub struct UpInfo {
    id: i64,
    column: String,
    info: String,
}
pub async fn admin_product_update_info(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(req): Json<UpInfo>,
) -> Result<Res, AeError> {
    let (column_info, column_count) = match &req.column[..] {
        "sale_info" => ("sale_info", "sale_count"),
        "stock_info" => ("stock_info", "stock_count"),
        _ => {
            return err("错误的更新请求字段".to_string());
        }
    };
    let mut count: i64 = 0; //总量
    let info: HashMap<String, HashMap<String, i64>> = serde_json::from_str(&req.info)?;
    for m in info.values() {
        for n in m.values() {
            count += n;
        }
    }
    if query(&format!(
        "update products set {}=?,{}=? where id=?",
        column_info, column_count
    ))
    .bind(req.info)
    .bind(count)
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

pub async fn admin_product_discount(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path((id, discount)): Path<(i64, i64)>,
) -> Result<Res, AeError> {
    let discount = if discount > 50 { 50 } else { discount }; //最大50%折扣
    if query("update products set discount=? where id=?")
        .bind(discount)
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

pub async fn admin_product_dl_discount_xslx(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
    Path(default_discount): Path<i64>,
) -> Result<Response, AeError> {
    use rust_xlsxwriter::{Format, Workbook};
    let mut workbook = Workbook::new();

    let worksheet = workbook.add_worksheet();
    //sheet名字
    worksheet.set_name("product_import.csv")?;

    //设置头
    worksheet.set_column_width(0, 20)?;
    worksheet.write(0, 0, "Product ID")?;
    worksheet.write(0, 1, "Product Title")?;
    worksheet.write(0, 2, "Discount")?;
    worksheet.write(0, 3, "Target People")?;
    worksheet.write(0, 4, "Extra Discount")?;
    worksheet.write(0, 5, "Limit Buy Per Customer")?;
    worksheet.write(0, 6, "p_id")?;

    //数字格式
    let decimal_format = Format::new().set_num_format("###0");
    query("select p.id,p.product_id,p.discount as adjust,o.discount as base from products p left join offers o on p.offer_id=o.offer_id where p.deleted_at is null AND o.deleted_at is null").fetch_all(&db)
    .await?
    .iter()
    .enumerate()
    .for_each(|(i,row)| {
        let product_id:i64 = row.get("product_id");
        let base:i64 = row.get("base");
        let adjust:i64 = row.get("adjust");
        let discount:i64 = 100 - (100 - base) * (100 - adjust) * (100 - default_discount) / 10000;
        let id:i64 = row.get("id");
        //row
        worksheet.write(i as u32 + 1, 0, product_id.to_string()).unwrap();
        worksheet.write(i as u32 + 1, 1, "").unwrap();
        worksheet.write_with_format(i as u32 + 1, 2, discount, &decimal_format).unwrap();
        worksheet.write(i as u32 + 1, 3, "").unwrap();
        worksheet.write(i as u32 + 1, 4, "").unwrap();
        worksheet.write(i as u32 + 1, 5, "").unwrap();
        worksheet.write_with_format(i as u32 + 1, 6, id, &decimal_format).unwrap();
    });

    let xlsx_name = &format!("product_discount-{}.xlsx", default_discount);
    let tmp_dir = settings["TMP_DIR"].as_str().unwrap_or("tmp");
    let file_path = PathBuf::from(tmp_dir).join(xlsx_name);

    workbook.save(&file_path)?; //最后保存文件

    debug!("created discount file, path: {:?}", &file_path);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", xlsx_name),
        )
        .header(
            "Content-Type",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .body(Body::from(fs::read(&file_path)?))?)
}

pub async fn admin_product_upload_xlsx(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
    mut multipart: Multipart,
) -> Result<Res, AeError> {
    let pid_title = settings["XLSX_PID_COLUMN_TITLE"]
        .as_str()
        .unwrap_or("|商品ID|");
    let uv30_title = settings["XLSX_UV30_COLUMN_TITLE"]
        .as_str()
        .unwrap_or("|访客数|");
    let sales30_title = settings["XLSX_SALES30_COLUMN_TITLE"]
        .as_str()
        .unwrap_or("|支付商品件数|");
    let barrier_uv30 = settings["UNPUBLISH_BARRIER_UV30"].as_i64().unwrap_or(10);
    let now = OffsetDateTime::now_local()?;
    let today = now.date();
    let days_before = now - Duration::days(settings["ANALYSIS_BEFORE"].as_i64().unwrap_or(180));
    let sql_str = "update products set uv30=?,sales30=?,sale_record=?,updated_at=? where id=?";
    let sql_str_2 =
        "update products set uv30=?,sales30=?,sale_record=?,updated_at=?,pending=-2 where id=?";

    let reg = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();

    while let Some(field) = multipart.next_field().await? {
        let file_name = field.file_name().unwrap().to_string();
        let date = match reg.find(&file_name) {
            Some(m) => m.as_str(),
            None => {
                return err("file name not contain date".to_string());
            }
        };
        let data = field.bytes().await?;
        let file_path =
            PathBuf::from(settings["TMP_DIR"].as_str().unwrap_or("tmp")).join(&file_name);
        fs::write(&file_path, data)?;

        let mut workbook: Xlsx<_> = open_workbook(&file_path)?;
        let sheets = workbook.sheet_names().to_owned();

        if let Ok(sheet_rows) = workbook.worksheet_range(&sheets[0]) {
            let mut pid_i = 0; //记录pid所在列
            let mut uv30_i = 0; //记录uv30所在列
            let mut sales30_i = 0; //记录sales30所在列
            let mut records: HashMap<i64, (String, i64, i64)> = HashMap::new();

            for (i, row) in sheet_rows.rows().enumerate() {
                if i == 0 {
                    for (j, title) in row.iter().enumerate() {
                        match title {
                            Data::String(s) => {
                                let s = "|".to_string() + s + "|";
                                if pid_title.contains(&s) {
                                    //第j列是pid
                                    pid_i = j;
                                } else if uv30_title.contains(&s) {
                                    //第j列是uv30
                                    uv30_i = j;
                                } else if sales30_title.contains(&s) {
                                    //第j列是sales30
                                    sales30_i = j;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    let pid = row[pid_i]
                        .get_string()
                        .unwrap_or("0")
                        .parse::<i64>()
                        .unwrap_or(0);
                    let uv = row[uv30_i]
                        .get_string()
                        .unwrap_or("0")
                        .parse::<i64>()
                        .unwrap_or(0);
                    let sale = row[sales30_i]
                        .get_string()
                        .unwrap_or("0")
                        .parse::<i64>()
                        .unwrap_or(0);

                    records.insert(pid, (date.to_string(), uv, sale));
                }
            }

            let mut current_id = 0;
            let max_id: (i64,) = query_as(
                "select id from products where deleted_at is null order by id desc limit 1",
            )
            .fetch_one(&db)
            .await?;
            let max_id = max_id.0;
            while current_id < max_id {
                let rows = query("select id,product_id,sale_record,created_at from products where id>? and deleted_at is null order by id asc limit 50").bind(current_id).fetch_all(&db).await?;
                for row in rows {
                    let id: i64 = row.get("id");
                    current_id = max(current_id, id);
                    let product_id: i64 = row.get("product_id");
                    let mut sale_record_str: String = row.get("sale_record");
                    let created_at = OffsetDateTime::parse(row.get("created_at"), &Rfc3339)?;
                    let sale_record_this_day: Value =
                        if let Some((date, uv, sale)) = records.get(&product_id) {
                            json!({
                                "date": date,
                                "sale": sale,
                                "uv": uv
                            })
                        } else {
                            json!({
                                "date": date,
                                "sale": 0,
                                "uv": 0
                            })
                        };
                    let mut sale_record = from_str::<Value>(&sale_record_str)
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .to_owned();
                    if sale_record.len() == 0
                        || sale_record[sale_record.len() - 1]["date"].as_str().unwrap() != date
                    {
                        sale_record.push(sale_record_this_day);
                        while sale_record.len() > 400 {
                            sale_record.remove(0);
                        }
                    }
                    sale_record_str = json!(sale_record).to_string();
                    let (uv30, sales30) = match sale_record.rchunks(30).next() {
                        Some(recent30) => {
                            let mut u = 0;
                            let mut s = 0;
                            recent30.iter().for_each(|v| {
                                u += v["uv"].as_i64().unwrap_or(0);
                                s += v["sale"].as_i64().unwrap_or(0);
                            });
                            (u, s)
                        }
                        None => (0, 0),
                    };

                    if created_at < days_before && uv30 < barrier_uv30 {
                        query(sql_str_2)
                    } else {
                        query(sql_str)
                    }
                    .bind(uv30)
                    .bind(sales30)
                    .bind(sale_record_str)
                    .bind(now)
                    .bind(id)
                    .execute(&db)
                    .await?;
                }
            }
        }
    }

    //删除180天前废弃的products
    query("delete from products where deleted_at is not null and deleted_at < ?")
        .bind(today - Duration::days(180))
        .execute(&db)
        .await?;

    return ok(json!(()));
}

pub async fn admin_product_available(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
) -> Result<Res, AeError> {
    query("update products set pending=0 where pending=-2 and deleted_at is null")
        .execute(&db)
        .await?;
    return ok(json!(()));
}
