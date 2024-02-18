CREATE TABLE offers(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    offer_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 1688 offer id
    product_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 对应ae商品ID
    title TEXT NOT NULL DEFAULT '', -- 标题
    cover TEXT NOT NULL DEFAULT '', -- 主图
    wireless_video_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 无线视频ID
    detail_video_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 详情视频ID
    model_id CHARACTER(16) NOT NULL DEFAULT '', -- 商家型号
    sale30 INTEGER NOT NULL DEFAULT 0, -- 商家月?销量
    sale_record TEXT NOT NULL DEFAULT '[]', -- 商家400天销量记录,json格式,[{"date":"2020-01-01","count":100},{"date":"2020-01-02","count":200}]
    sale_info TEXT NOT NULL DEFAULT '', -- 商家销量统计
    price INTEGER NOT NULL DEFAULT 0, -- 原价，人民币
    better_price INTEGER NOT NULL DEFAULT 0, -- 现价，人民币
    discount INTEGER NOT NULL DEFAULT 0, -- 折扣率
    sku_info TEXT NOT NULL DEFAULT '', -- 现sku信息
    sku_info_use TEXT NOT NULL DEFAULT '', -- 正在使用中的sku信息，有可能过时，处理信息变更后同步sku_info
    detail_url VARCHAR(255) NOT NULL DEFAULT '', -- 商品详情页面
    detail_url_use VARCHAR(255) NOT NULL DEFAULT '', -- 正在使用中的商品详情页面，有可能过时，处理信息变更后同步detail_url
    supplier VARCHAR(128) NOT NULL DEFAULT '', -- 供货商
    store_url VARCHAR(255) NOT NULL DEFAULT '', -- 店铺地址
    pending INTEGER NOT NULL DEFAULT -2, -- 是否需要处理信息变更(-2未发布, -1待处理更新)
    tips TEXT NOT NULL DEFAULT '', -- 提示，备注

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 更新时间(打开1688详情页自动更新)
    deleted_at TIMESTAMP, -- 删除时间，大于零则为删除
    promotion_end TIMESTAMP -- 活动结束时间
);
CREATE INDEX offers_deleted_at on offers (deleted_at);
CREATE UNIQUE INDEX offers_offer_id on offers (offer_id);
CREATE INDEX offers_pending on offers (pending);
CREATE INDEX offers_product_id on offers (product_id);
CREATE INDEX offers_updated_at on offers (updated_at);

CREATE TABLE products(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    product_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 商品ID
    uv30 INTEGER NOT NULL DEFAULT 0, -- 30天浏览人数
    sales30 INTEGER NOT NULL DEFAULT 0, -- 30内销量
    offer_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 1688 offer id
    title CHARACTER(128) NOT NULL DEFAULT '', -- 标题
    cover TEXT NOT NULL DEFAULT '', -- 主图
    price INTEGER NOT NULL DEFAULT 0, -- 价格，美元
    discount INTEGER NOT NULL DEFAULT 0, -- 在基础折扣上调整的折扣率
    stock_count INTEGER NOT NULL DEFAULT 0, -- 库存总量，通过stock_info统计
    stock_info TEXT NOT NULL DEFAULT '', -- 库存，json格式 {size:{color: num}}
    sale_count INTEGER NOT NULL DEFAULT 0, -- 已卖出数量
    sale_info TEXT NOT NULL DEFAULT '', -- 卖出计量，格式同库存
    sale_weight UNSIGNED BIG INT NOT NULL DEFAULT 0, -- 已卖出总重量
    weight_cal_count INTEGER NOT NULL DEFAULT 0, -- 用于计算重量的卖出数
    weight INTEGER NOT NULL DEFAULT 0, -- 建议重量
    inited_weight INTEGER NOT NULL DEFAULT 0, -- 是否已初始化重量(1是0否)
    model_id CHARACTER(16) NOT NULL DEFAULT '', -- 型号
    pending INTEGER NOT NULL DEFAULT 0, -- 是否需要处理信息变更(-1待处理重量变化, -2待下架)
    tips TEXT NOT NULL DEFAULT '', -- 提示，备注

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 下架分析遍历时间
    deleted_at TIMESTAMP -- 删除时间，大于零则为删除
);
CREATE INDEX products_created_at on products (created_at);
CREATE INDEX products_updated_at on products (updated_at);
CREATE INDEX products_deleted_at on products (deleted_at);
CREATE INDEX products_offer_id on products (offer_id);
CREATE UNIQUE INDEX products_product_id on products (product_id);

CREATE TABLE orders (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    order_id UNSIGNED BIG INT NOT NULL DEFAULT 0, -- ae order id
    lg_order_id CHARACTER(20), -- 物流单号
    remark TEXT NOT NULL DEFAULT '', -- 订单备注
    weight INTEGER NOT NULL DEFAULT 0, -- 包裹重量
    product_num INTEGER NOT NULL DEFAULT 1, -- 商品种类
    item_num INTEGER NOT NULL DEFAULT 1, -- 商品总数
    products TEXT NOT NULL DEFAULT '', -- 产品ID及数量
    used_stock TEXT NOT NULL DEFAULT '', -- 订单内使用了库存的项{"product_id-color-size":quantity}
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 创建时间
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP -- 物流单遍历更新重量的时间
);
CREATE UNIQUE INDEX orders_order_id on orders (order_id);
CREATE INDEX orders_updated_at on orders (updated_at);
CREATE INDEX orders_created_at on orders (created_at);