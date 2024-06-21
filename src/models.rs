#![allow(dead_code, unused_imports, unused)]

use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use sqlx::FromRow;
use std::cmp::{max, min};
use time::{
    serde::rfc3339::{self as show_time, option as show_option_time},
    Duration, OffsetDateTime,
};

//serde serialize 时间格式
//format_description!(show_time, OffsetDateTime,"[year]-[month]-[day] [hour]:[minute]:[second]");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewOffer {
    pub offer_id: i64,
    pub title: String,
    pub cover: String,
    pub wireless_video_id: i64,
    pub detail_video_id: i64,
    pub model_id: String,
    pub sale30: i64,
    pub sale_info: String,
    pub price: i64,
    pub better_price: i64,
    pub sku_info: String,
    pub detail_url: String,
    pub supplier: String,
    pub store_url: String,
    #[serde(with = "show_option_time")]
    pub promotion_end: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Offer {
    pub id: Option<i64>,
    pub product_id: i64,
    pub sale_record: String,
    pub discount: i64,
    pub sku_info_use: String,
    pub detail_url_use: String,
    pub pending: i64,
    pub tips: String,
    #[serde(with = "show_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "show_time")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "show_option_time")]
    pub deleted_at: Option<OffsetDateTime>,

    //NewOffer
    pub offer_id: i64,
    pub title: String,
    pub cover: String,
    pub wireless_video_id: i64,
    pub detail_video_id: i64,
    pub model_id: String,
    pub sale30: i64,
    pub sale_info: String,
    pub price: i64,
    pub better_price: i64,
    pub sku_info: String,
    pub detail_url: String,
    pub supplier: String,
    pub store_url: String,
    #[serde(with = "show_option_time")]
    pub promotion_end: Option<OffsetDateTime>,
}
impl Offer {
    pub fn new(no: &NewOffer) -> Self {
        let can_booked_amount: Value = from_str(&no.sale_info).unwrap();
        let mut sale_record: Vec<Value> = vec![
            json!({
                "date": OffsetDateTime::now_local().unwrap().date().to_string(),
                "count": 0}),
            can_booked_amount.clone(),
        ];

        let mut sale_info = can_booked_amount.clone();
        //新增的offer销量刚开始统计为0
        sale_info["color"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .for_each(|mut v| {
                *v = json!(0);
            });
        sale_info["size"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .for_each(|mut v| {
                *v = json!(0);
            });
        sale_info["detail"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .for_each(|mut v| {
                *v = json!(0);
            });

        Self {
            id: None,
            product_id: 0,
            sale_record: json!(sale_record).to_string(),
            discount: (no.price - no.better_price) * 100 / no.price,
            sku_info_use: no.sku_info.clone(),
            detail_url_use: no.detail_url.clone(),
            pending: -2,
            tips: String::from("!草稿箱;"),
            created_at: OffsetDateTime::now_local().unwrap(),
            updated_at: OffsetDateTime::now_local().unwrap(),
            deleted_at: None,

            //NewOffer
            offer_id: no.offer_id,
            title: no.title.clone(),
            cover: no.cover.clone(),
            wireless_video_id: no.wireless_video_id,
            detail_video_id: no.detail_video_id,
            model_id: no.model_id.clone(),
            sale30: 0, //新增的offer销量刚开始统计为0
            sale_info: sale_info.to_string(),
            price: no.price,
            better_price: no.better_price,
            sku_info: no.sku_info.clone(),
            detail_url: no.detail_url.clone(),
            supplier: no.supplier.clone(),
            store_url: no.store_url.clone(),
            promotion_end: no.promotion_end.clone(),
        }
    }

    pub fn update(mut self, no: &NewOffer, cfg: Value) -> Self {
        self.updated_at = OffsetDateTime::now_local().unwrap();
        let today = self.updated_at.date().to_string();

        self.title = no.title.clone();
        self.cover = no.cover.clone();
        //详情视频变更
        if self.detail_video_id != no.detail_video_id {
            self.detail_video_id = no.detail_video_id;
            self.tips += "详情视频变更;";
            if self.pending == 0 {
                self.pending = -1;
            }
        }
        //无线视频变更
        if self.wireless_video_id != no.wireless_video_id {
            self.wireless_video_id = no.wireless_video_id;
            self.tips += "无线视频变更;";
            if self.pending == 0 {
                self.pending = -1;
            }
        }
        //self.model_id = no.model_id.clone();

        let mut sale_info: Value = from_str(&self.sale_info).unwrap();
        let mut records = from_str::<Value>(&self.sale_record)
            .unwrap()
            .as_array()
            .unwrap()
            .to_owned();
        let can_book_amount: Value = from_str(&no.sale_info).unwrap();
        let prev_can_book_amount = records.pop().unwrap_or(can_book_amount.clone());
        if records.len() == 0 || records[0]["date"].as_str().unwrap() != &today {
            can_book_amount["color"]
                .as_object()
                .unwrap()
                .iter()
                .for_each(|(k, v)| {
                    let v = v.as_i64().unwrap();
                    let sale_count = match prev_can_book_amount["color"][k].as_i64() {
                        Some(p) => {
                            //之前有这个颜色，现在也还有这个颜色
                            min(max(p - v, 0), 500) //销量在0-500之间才可信
                        }
                        None => {
                            //之前没有这个颜色
                            0
                        }
                    };
                    match sale_info["color"][k].as_i64() {
                        Some(p) => {
                            //销量记录里有这个颜色
                            sale_info["color"][k] = json!(p + sale_count);
                        }
                        None => {
                            //销量记录里没有这个颜色
                            sale_info["color"][k] = json!(sale_count);
                        }
                    }
                });
            can_book_amount["size"]
                .as_object()
                .unwrap()
                .iter()
                .for_each(|(k, v)| {
                    let v = v.as_i64().unwrap();
                    let sale_count = match prev_can_book_amount["size"][k].as_i64() {
                        Some(p) => {
                            //之前有这个尺码，现在也还有这个尺码
                            min(max(p - v, 0), 500) //销量在0-500之间才可信
                        }
                        None => {
                            //之前没有这个尺码
                            0
                        }
                    };
                    match sale_info["size"][k].as_i64() {
                        Some(p) => {
                            //销量记录里有这个尺码
                            sale_info["size"][k] = json!(p + sale_count);
                        }
                        None => {
                            //销量记录里没有这个尺码
                            sale_info["size"][k] = json!(sale_count);
                        }
                    }
                });
            let mut sale_today = 0;
            can_book_amount["detail"]
                .as_object()
                .unwrap()
                .iter()
                .for_each(|(k, v)| {
                    let v = v.as_i64().unwrap();
                    let sale_count = match prev_can_book_amount["detail"][k].as_i64() {
                        Some(p) => {
                            //之前有这个sku，现在也还有这个sku
                            min(max(p - v, 0), 200) //销量在0-200之间才可信
                        }
                        None => {
                            //之前没有这个sku
                            0
                        }
                    };
                    sale_today += sale_count;
                    match sale_info["detail"][k].as_i64() {
                        Some(p) => {
                            //销量记录里有这个sku
                            sale_info["detail"][k] = json!(p + sale_count);
                        }
                        None => {
                            //销量记录里没有这个sku
                            sale_info["detail"][k] = json!(sale_count);
                        }
                    }
                });
            let sale_record_today = json!({
                "date": &today,
                "count": sale_today
            });

            records.insert(0, sale_record_today);

            while records.len() > 400 {
                records.pop().unwrap();
            }
        }
        self.sale30 = match records.chunks(30).next() {
            Some(recent30) => {
                let mut sale30 = 0;
                recent30.iter().for_each(|v| {
                    sale30 += v["count"].as_i64().unwrap();
                });
                sale30
            }
            None => 0,
        };

        //月销量低的下架？
        let sale60 = match records.chunks(60).next() {
            Some(recent60) => {
                let mut sale60 = 0;
                recent60.iter().for_each(|v| {
                    sale60 += v["count"].as_i64().unwrap();
                });
                sale60
            }
            None => 0,
        };
        if self.updated_at - self.created_at
            > Duration::days(cfg["CHECK_OFFER_SALES_AFTER_DAYS"].as_i64().unwrap_or(90))
        {
            if sale60 < (sale_info["detail"].as_object().unwrap().len() as i64) {
                //销量小于sku数
                self.tips += "销量低下架否?;";
                if self.pending == 0 {
                    self.pending = -1;
                }
            }
        }

        records.push(can_book_amount);
        self.sale_record = json!(records).to_string();

        self.sale_info = sale_info.to_string();

        self.detail_url = no.detail_url.clone();

        //price不更新
        if self.better_price != no.better_price {
            self.better_price = no.better_price;
            self.tips += &self.discount.to_string();
            self.discount = (self.price - self.better_price) * 100 / self.price;
            self.tips += &(" => ".to_string() + &self.discount.to_string() + " 折扣价变更;");
            if self.pending == 0 {
                self.pending = -1;
            }
        }
        if no.better_price > self.price {
            self.tips += "手动提价！！;";
            if self.pending == 0 {
                self.pending = -1;
            }
        }
        //sku_info_use保持原样
        if &self.sku_info_use != &no.sku_info {
            self.sku_info = no.sku_info.clone();
            self.tips += "SKU变更;";
            if self.pending == 0 {
                self.pending = -1;
            }
        }

        //detail_url_use保持原样
        if &self.detail_url_use != &no.detail_url {
            self.detail_url = no.detail_url.clone();
            self.tips += "详情链接变更;";
            if self.pending == 0 {
                self.pending = -1;
            }
        }
        self.supplier = no.supplier.clone();
        self.store_url = no.store_url.clone();
        self.promotion_end = no.promotion_end.clone();

        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewProduct {
    pub product_id: i64,
    pub title: String,
    pub cover: String,
    pub price: i64,
    pub stock_info: String,
    pub model_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Product {
    pub id: Option<i64>,
    pub uv30: i64,
    pub sales30: i64,
    pub sale_record: String,
    pub offer_id: i64,
    pub discount: i64,
    pub stock_count: i64,
    pub sale_count: i64,
    pub sale_info: String,
    pub sale_weight: i64,
    pub weight_cal_count: i64,
    pub weight: i64,
    pub inited_weight: i64,
    pub pending: i64,
    pub tips: String,
    #[serde(with = "show_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "show_time")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "show_option_time")]
    pub deleted_at: Option<OffsetDateTime>,

    //NewProduct
    pub product_id: i64,
    pub title: String,
    pub cover: String,
    pub price: i64,
    pub stock_info: String,
    pub model_id: String,
}

impl Product {
    pub fn new(np: &NewProduct) -> Self {
        Self {
            id: None,
            uv30: 0,
            sales30: 0,
            sale_record: "[]".to_string(),
            offer_id: 0,
            discount: 0,
            stock_count: 0,
            sale_count: 0,
            sale_info: np.stock_info.clone(),
            sale_weight: 0,
            weight_cal_count: 0,
            weight: 0,
            inited_weight: 0,
            pending: 0,
            tips: String::new(),
            created_at: OffsetDateTime::now_local().unwrap(),
            updated_at: OffsetDateTime::now_local().unwrap(),
            deleted_at: None,

            //NewProduct
            product_id: np.product_id,
            title: np.title.clone(),
            cover: np.cover.clone(),
            price: np.price,
            stock_info: np.stock_info.clone(),
            model_id: np.model_id.clone(),
        }
    }

    pub fn update(mut self, np: &NewProduct) -> Self {
        self.title = np.title.clone();
        self.cover = np.cover.clone();
        self.price = np.price;
        if self.stock_info.len() < 1 && np.stock_info.len() > 1 {
            self.stock_info = np.stock_info.clone();
            self.sale_info = np.stock_info.clone();
        }
        self.model_id = np.model_id.clone();
        self.updated_at = OffsetDateTime::now_local().unwrap();

        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOrder {
    pub order_id: i64,
    pub remark: String,
    pub product_num: i64,
    pub item_num: i64,
    pub products: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Order {
    pub id: Option<i64>,
    pub lg_order_id: Option<String>,
    pub weight: i64,
    pub used_stock: String,
    #[serde(with = "show_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "show_time")]
    pub updated_at: OffsetDateTime,

    //NewOrder
    pub order_id: i64,
    pub remark: String,
    pub product_num: i64,
    pub item_num: i64,
    pub products: String,
}

impl Order {
    pub fn new(no: &NewOrder) -> Self {
        Self {
            id: None,
            lg_order_id: None,
            weight: 0,
            used_stock: String::new(),
            created_at: OffsetDateTime::now_local().unwrap(),
            updated_at: OffsetDateTime::now_local().unwrap(),

            //NewOrder
            order_id: no.order_id,
            remark: no.remark.clone(),
            product_num: no.product_num,
            item_num: no.item_num,
            products: no.products.clone(),
        }
    }

    pub fn update(&mut self, no: &NewOrder) {
        self.remark = no.remark.clone();
        self.updated_at = OffsetDateTime::now_local().unwrap();
    }
}
