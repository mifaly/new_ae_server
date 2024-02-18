use crate::models::{NewOrder, Order, Product};
use crate::types::{err, ok, AEState, AeError, Res};
use anyhow::anyhow;
use axum::extract::{Json, Path, State};
use serde::Deserialize;
use serde_json::json;
use sqlx::{query, query_as, QueryBuilder};
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use time::{Duration, OffsetDateTime};
use tracing::error;

pub async fn get_from_order_id(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Path(oid): Path<i64>,
) -> Result<Res, AeError> {
    let order_: Option<Order> = query_as("SELECT * FROM orders WHERE order_id = ?1")
        .bind(oid)
        .fetch_optional(&db)
        .await?;
    if let Some(order) = order_ {
        return ok(json!(order));
    } else {
        return err("not found".to_string());
    }
}

pub async fn update_or_add(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(patch_orders): Json<HashMap<String, NewOrder>>,
) -> Result<Res, AeError> {
    let order_ids: HashSet<i64> = patch_orders
        .keys()
        .map(|order_id| order_id.parse().unwrap_or(0))
        .filter(|i| *i > 0) //过滤掉不适当的值，上一步已置为0
        .collect();

    let mut exist_orders: Vec<Order> = if order_ids.len() > 0 {
        let query_str = format!(
            "select * from orders where order_id in ({})",
            vec!["?"; order_ids.len()].join(",")
        );
        let mut query_ = query_as(&query_str);
        for order_id in order_ids.iter() {
            query_ = query_.bind(order_id);
        }
        query_.fetch_all(&db).await?
    } else {
        Vec::new()
    };

    // 开始事务
    let mut db_trans = db.begin().await?;

    for order in exist_orders.iter_mut() {
        order.update(&patch_orders.get(&order.order_id.to_string()).unwrap());
        let affacted_rows = query("update orders set remark=?,updated_at=? where order_id=?")
            .bind(&order.remark)
            .bind(order.updated_at)
            .bind(order.order_id)
            .execute(&mut *db_trans)
            .await?
            .rows_affected();
        if affacted_rows == 0 {
            db_trans.rollback().await?;
            error!("orders未更新, 请手动检查: order_id: {}", order.order_id);
            return err("orders未更新, 请手动检查".to_string());
        }
    }

    let new_orders: Vec<Order> = order_ids
        .difference(&exist_orders.iter().map(|o| o.order_id).collect())
        .map(|i| Order::new(patch_orders.get(&i.to_string()).unwrap()))
        .collect();
    if new_orders.len() > 0 {
        let mut query_builder =
            QueryBuilder::new("INSERT INTO orders(lg_order_id, weight, used_stock, created_at, updated_at, order_id, remark, product_num, item_num, products) ");
        query_builder.push_values(&new_orders, |mut b, order| {
            b.push_bind(&order.lg_order_id)
                .push_bind(order.weight)
                .push_bind(&order.used_stock)
                .push_bind(order.created_at)
                .push_bind(order.updated_at)
                .push_bind(order.order_id)
                .push_bind(&order.remark)
                .push_bind(order.product_num)
                .push_bind(order.item_num)
                .push_bind(&order.products);
        });
        let affacted_rows = query_builder
            .build()
            .execute(&mut *db_trans)
            .await?
            .rows_affected();
        if affacted_rows != new_orders.len() as u64 {
            db_trans.rollback().await?;
            error!("orders未添加, 请手动检查！");
            return err("orders未添加, 请手动检查！".to_string());
        }
    }

    // 提交事务
    db_trans.commit().await?;

    return ok(json!(exist_orders));
}

pub async fn next(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
) -> Result<Res, AeError> {
    let order_: Option<(Option<String>,)> = query_as("select lg_order_id from orders where created_at between ?1 and ?2 and updated_at < ?3 and weight = 0 and lg_order_id is not null order by id asc limit 1")
    .bind(OffsetDateTime::now_local()?.date() - Duration::days(60))
    .bind(OffsetDateTime::now_local()?.date() - Duration::days(3))
    .bind(OffsetDateTime::now_local()?.date())
    .fetch_optional(&db).await?;
    if let Some((Some(lg_order_id),)) = order_ {
        if let Some(lg_order_url_pattern) = settings["LG_ORDER_URL_PATTERN"].as_str() {
            return ok(json!(
                lg_order_url_pattern.replace("{LG_ORDER_ID}", &lg_order_id)
            ));
        } else {
            return Err(anyhow!("no lg_order_url_pattern").into());
        }
    } else {
        //删除180天前的订单
        query("delete from orders where created_at < ?")
            .bind(OffsetDateTime::now_local()?.date() - Duration::days(180))
            .execute(&db)
            .await?;

        return err("all done".to_string());
    }
}

#[derive(Deserialize)]
pub struct UpOdLg {
    order_id: i64,
    lg_order_id: String,
}
pub async fn set_lg_id(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(sets): Json<Vec<UpOdLg>>,
) -> Result<Res, AeError> {
    for o in sets.iter() {
        //不用考虑更新失败，因为重入页面不符合“where”更新条件，不会发生更新
        query("update orders set lg_order_id=?,updated_at=? where order_id=? and (lg_order_id is null or lg_order_id<?)").bind(&o.lg_order_id).bind(OffsetDateTime::now_local()?).bind(o.order_id).bind(&o.lg_order_id).execute(&db).await?;
    }

    let query_str = format!(
        "select * from orders where order_id in ({})",
        vec!["?"; sets.len()].join(",")
    );
    let mut query_ = query_as(&query_str);
    for o in sets.iter() {
        query_ = query_.bind(o.order_id);
    }
    let orders: Vec<Order> = query_.fetch_all(&db).await?;

    return ok(json!(orders));
}

pub async fn update_weight(
    State(AEState {
        db_pool: db,
        settings,
    }): State<AEState>,
    Path((oid, weight, item_num)): Path<(i64, i32, i32)>,
) -> Result<Res, AeError> {
    let order_: Option<Order> = query_as("SELECT * FROM orders WHERE order_id = ?1")
        .bind(oid)
        .fetch_optional(&db)
        .await?;
    if order_.is_none() {
        return err("未找到该订单".to_string());
    }
    let order = order_.unwrap();
    if order.weight > 0 {
        return err("已统计".to_string());
    }
    let affacted_rows = query("update orders set weight=?,updated_at=? where order_id=?")
        .bind(weight)
        .bind(OffsetDateTime::now_local()?)
        .bind(oid)
        .execute(&db)
        .await?
        .rows_affected();
    if affacted_rows == 0 {
        return err("未能更新该订单".to_string());
    }

    if order.product_num != 1 || item_num != order.item_num {
        return ok(json!("多商品或分包订单无法统计重量"));
    }

    let one_product_id = if let Some(pid) =
        serde_json::from_str::<HashMap<i64, Vec<(String, i32)>>>(&order.products)?
            .keys()
            .next()
    {
        pid.clone()
    } else {
        return err("未找到对应的product".to_string());
    };

    let product_: Option<Product> = query_as("SELECT * FROM products WHERE product_id = ?1")
        .bind(one_product_id)
        .fetch_optional(&db)
        .await?;
    if product_.is_none() {
        return err("未能获取该订单的产品".to_string());
    }
    let mut pd = product_.unwrap();

    let orig_weight_cal_count = pd.weight_cal_count;

    pd.weight_cal_count += order.item_num;
    pd.sale_weight += weight as i64;

    let weight_ratio = if let Some(wr) = settings["WEIGHT_RATIO"].as_i64() {
        wr
    } else {
        return Err(anyhow!("no weight_ratio").into());
    };
    pd.weight = ((pd.sale_weight / (pd.weight_cal_count as i64)) * 1000 / weight_ratio) as i32;

    let need_update_weight = if let Some(nuw) = settings["NEED_UPDATE_WEIGHT"].as_i64() {
        nuw as i32
    } else {
        return Err(anyhow!("no need_update_weight").into());
    };

    if pd.weight_cal_count == 1
        || pd.weight_cal_count < 33
            && (pd.weight_cal_count as f64).log2() as i32
                - (orig_weight_cal_count as f64).log2() as i32
                > 0
        || pd.weight_cal_count / need_update_weight - orig_weight_cal_count / need_update_weight > 0
    {
        //1,2,4,8,16,32和每32件,需要处理重量
        pd.pending = -1;
    }

    let affacted_rows = query("update products set weight_cal_count=?,sale_weight=?,weight=?,pending=? where product_id=?")
                    .bind(pd.weight_cal_count)
                    .bind(pd.sale_weight)
                    .bind(pd.weight)
                    .bind(pd.pending)
                    .bind(pd.product_id)
                    .execute(&db)
                    .await?
                    .rows_affected();
    if affacted_rows == 0 {
        return err("未能更新该订单所含产品".to_string());
    } else {
        return ok(json!({}));
    }
}

#[derive(Deserialize)]
pub struct SOReq {
    page: i64,
    per_page: i64,
    order_id: i64,
    product_id: i64,
}
pub async fn admin_order_show(
    State(AEState {
        db_pool: db,
        settings: _,
    }): State<AEState>,
    Json(mut search): Json<SOReq>,
) -> Result<Res, AeError> {
    let mut total_query_builder = QueryBuilder::new("select count(id) from orders where 1=1 ");
    let mut orders_query_builder = QueryBuilder::new("select * from orders where 1=1 ");

    if search.order_id > 0 {
        total_query_builder.push(" and order_id = ");
        total_query_builder.push_bind(search.order_id);
        orders_query_builder.push(" and order_id = ");
        orders_query_builder.push_bind(search.order_id);
    }
    if search.product_id > 0 {
        let like_str = format!("%{}%", search.product_id);
        total_query_builder.push(" and products like ");
        total_query_builder.push_bind(like_str.clone());
        orders_query_builder.push(" and products like ");
        orders_query_builder.push_bind(like_str.clone());
    }

    let total: (i64,) = total_query_builder.build_query_as().fetch_one(&db).await?;
    search.per_page = if search.per_page == 0 {
        20
    } else {
        search.per_page
    };
    search.page = max(1, min(search.page, total.0 / search.per_page + 1));
    orders_query_builder.push(" order by id desc");
    orders_query_builder.push(" limit ");
    orders_query_builder.push_bind(search.per_page);
    orders_query_builder.push(" offset ");
    orders_query_builder.push_bind((search.page - 1) * search.per_page);

    let orders: Vec<Order> = orders_query_builder.build_query_as().fetch_all(&db).await?;

    return ok(json!({
        "page": search.page,
        "per_page": search.per_page,
        "total": total.0,
        "orders": orders,
    }));
}
