#![allow(dead_code, unused_imports, unused)]

use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use sqlx::FromRow;
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
        Self {
            id: None,
            product_id: 0,
            sale_record: json!([
                {
                    "date": OffsetDateTime::now_local().unwrap().date().to_string(),
                    "count": no.sale30
                }
            ])
            .to_string(),
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
            sale_info: no.sale_info.clone(),
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
        let sale_record_today = json!({
            "date": &today,
            "count": no.sale30
        });
        let mut records = from_str::<Value>(&self.sale_record)
            .unwrap()
            .as_array()
            .unwrap()
            .to_owned();
        if records.len() == 0 || records[records.len() - 1]["date"].as_str().unwrap() != &today {
            records.push(sale_record_today);

            while records.len() > 400 {
                records.remove(0);
            }
        }
        self.sale_record = json!(records).to_string();
        self.sale30 = if records.len() < 2 {
            0
        } else if records.len() < 31 {
            records[records.len() - 1]["count"].as_i64().unwrap()
                - records[0]["count"].as_i64().unwrap()
        } else {
            records[records.len() - 1]["count"].as_i64().unwrap()
                - records[records.len() - 31]["count"].as_i64().unwrap()
        };
        self.sale_info = no.sale_info.clone();
        self.detail_url = no.detail_url.clone();

        //月销量低的下架？
        let sale60 = if records.len() < 2 {
            0
        } else if records.len() < 61 {
            records[records.len() - 1]["count"].as_i64().unwrap()
                - records[0]["count"].as_i64().unwrap()
        } else {
            records[records.len() - 1]["count"].as_i64().unwrap()
                - records[records.len() - 61]["count"].as_i64().unwrap()
        };
        if self.updated_at - self.created_at
            > Duration::days(cfg["CHECK_OFFER_SALES_AFTER_DAYS"].as_i64().unwrap_or(90))
        {
            let sale_info: Value = from_str(&self.sale_info).unwrap();
            let skus = sale_info["color"].as_array().unwrap().len()
                * sale_info["size"].as_array().unwrap().len();
            if sale60 < (skus as i64) {
                //销量小于sku数
                self.tips += "销量低下架否?;";
                if self.pending == 0 {
                    self.pending = -1;
                }
            }
        }

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
