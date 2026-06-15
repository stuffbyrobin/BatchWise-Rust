//! Data access for customers, orders, order items, and duty events.
//!
//! Port of the Go `internal/sales/repository.go`. `NUMERIC` columns are
//! selected as `float8`; `DATE` columns render with `to_char` and bind back via
//! `::date`. Money stays `i64` pence; `total_price_pence` on order items is a
//! generated column. Every query is tenant-scoped.

use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{
    Customer, CustomerFilter, DutyEvent, DutyEventFilter, Order, OrderFilter, OrderItem, Page,
};

const CUSTOMER_COLS: &str = "id, tenant_id, name, contact_name, email, phone, \
    address_line1, address_line2, city, postcode, country, notes, created_at, updated_at";

// `customer_name` and `total_price_pence` are computed at query time (neither is
// stored on `orders`). The trailing `FROM ... JOIN ...` is shared by list/get.
const ORDER_SELECT: &str = "SELECT o.id, o.tenant_id, o.customer_id, c.name AS customer_name, \
    o.order_number, o.status, to_char(o.order_date, 'YYYY-MM-DD') AS order_date, \
    to_char(o.fulfillment_date, 'YYYY-MM-DD') AS fulfillment_date, o.notes, \
    COALESCE((SELECT SUM(total_price_pence) FROM order_items WHERE order_id = o.id), 0)::bigint AS total_price_pence, \
    o.created_at, o.updated_at \
    FROM orders o JOIN customers c ON c.id = o.customer_id AND c.tenant_id = o.tenant_id";

const ITEM_COLS: &str = "id, tenant_id, order_id, batch_id, product_name, \
    volume_liters::float8 AS volume_liters, unit_price_pence, quantity, total_price_pence, \
    notes, created_at";

const DUTY_COLS: &str = "id, tenant_id, order_id, batch_id, event_type, \
    volume_liters::float8 AS volume_liters, abv_pct::float8 AS abv_pct, duty_pence, \
    jurisdiction, crystallised_at, created_at";

fn clamp_page(page: i64, page_size: i64) -> (i64, i64) {
    let page = if page < 1 { 1 } else { page };
    let page_size = if page_size < 1 {
        20
    } else if page_size > 100 {
        100
    } else {
        page_size
    };
    (page, page_size)
}

// ---- customers ----

/// Inserts a customer and returns the created row.
#[allow(clippy::too_many_arguments)]
pub async fn insert_customer(
    pool: &PgPool,
    tenant_id: Uuid,
    c: &Customer,
) -> Result<Customer, sqlx::Error> {
    let sql = format!(
        "INSERT INTO customers (tenant_id, name, contact_name, email, phone, \
         address_line1, address_line2, city, postcode, country, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING {CUSTOMER_COLS}"
    );
    sqlx::query_as::<_, Customer>(&sql)
        .bind(tenant_id)
        .bind(&c.name)
        .bind(&c.contact_name)
        .bind(&c.email)
        .bind(&c.phone)
        .bind(&c.address_line1)
        .bind(&c.address_line2)
        .bind(&c.city)
        .bind(&c.postcode)
        .bind(&c.country)
        .bind(&c.notes)
        .fetch_one(pool)
        .await
}

/// Fetches a customer by id, tenant-scoped.
pub async fn select_customer_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Customer>, sqlx::Error> {
    let sql = format!("SELECT {CUSTOMER_COLS} FROM customers WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Customer>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists customers with an optional name prefix filter.
pub async fn select_customers(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &CustomerFilter,
    order_by: &str,
) -> Result<Page<Customer>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let q = filter.q.clone().filter(|s| !s.is_empty());
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(s) = &q {
            qb.push(" AND name ILIKE ").push_bind(format!("{s}%"));
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM customers");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {CUSTOMER_COLS} FROM customers"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by} "));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Customer>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Updates all mutable customer fields and returns the updated row.
pub async fn update_customer(pool: &PgPool, c: &Customer) -> Result<Option<Customer>, sqlx::Error> {
    let sql = format!(
        "UPDATE customers SET name=$3, contact_name=$4, email=$5, phone=$6, \
         address_line1=$7, address_line2=$8, city=$9, postcode=$10, country=$11, notes=$12, \
         updated_at=now() WHERE tenant_id=$1 AND id=$2 RETURNING {CUSTOMER_COLS}"
    );
    sqlx::query_as::<_, Customer>(&sql)
        .bind(c.tenant_id)
        .bind(c.id)
        .bind(&c.name)
        .bind(&c.contact_name)
        .bind(&c.email)
        .bind(&c.phone)
        .bind(&c.address_line1)
        .bind(&c.address_line2)
        .bind(&c.city)
        .bind(&c.postcode)
        .bind(&c.country)
        .bind(&c.notes)
        .fetch_optional(pool)
        .await
}

/// Deletes a customer; returns true if a row was removed.
pub async fn delete_customer(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM customers WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if the customer has any non-cancelled orders.
pub async fn customer_has_active_orders(
    pool: &PgPool,
    tenant_id: Uuid,
    customer_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM orders WHERE tenant_id=$1 AND customer_id=$2 \
         AND status NOT IN ('cancelled'))",
    )
    .bind(tenant_id)
    .bind(customer_id)
    .fetch_one(pool)
    .await
}

// ---- orders ----

/// Inserts an order (within a transaction) and returns it with computed fields.
pub async fn insert_order(conn: &mut PgConnection, o: &Order) -> Result<Order, sqlx::Error> {
    // Insert, then re-select via ORDER_SELECT so customer_name / total are
    // populated consistently (the row is visible within the same transaction).
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO orders (tenant_id, customer_id, order_number, status, order_date, notes) \
         VALUES ($1,$2,$3,$4,$5::date,$6) RETURNING id",
    )
    .bind(o.tenant_id)
    .bind(o.customer_id)
    .bind(&o.order_number)
    .bind(&o.status)
    .bind(&o.order_date)
    .bind(&o.notes)
    .fetch_one(&mut *conn)
    .await?;

    let sql = format!("{ORDER_SELECT} WHERE o.tenant_id=$1 AND o.id=$2");
    let mut order = sqlx::query_as::<_, Order>(&sql)
        .bind(o.tenant_id)
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;
    order.items = Vec::new();
    Ok(order)
}

/// Fetches an order by id (with its items), tenant-scoped.
pub async fn select_order_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Order>, sqlx::Error> {
    let sql = format!("{ORDER_SELECT} WHERE o.tenant_id=$1 AND o.id=$2");
    let order = sqlx::query_as::<_, Order>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match order {
        None => Ok(None),
        Some(mut o) => {
            o.items = select_items_by_order_id(pool, tenant_id, id).await?;
            Ok(Some(o))
        }
    }
}

/// Lists orders with filters (items omitted on list rows).
pub async fn select_orders(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &OrderFilter,
    order_by: &str,
) -> Result<Page<Order>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let status = filter.status.clone().filter(|s| !s.is_empty());
    let from_date = filter.from_date.clone().filter(|s| !s.is_empty());
    let to_date = filter.to_date.clone().filter(|s| !s.is_empty());
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE o.tenant_id = ").push_bind(tenant_id);
        if let Some(c) = filter.customer_id {
            qb.push(" AND o.customer_id = ").push_bind(c);
        }
        if let Some(s) = &status {
            qb.push(" AND o.status = ").push_bind(s.clone());
        }
        if let Some(f) = &from_date {
            qb.push(" AND o.order_date >= ")
                .push_bind(f.clone())
                .push("::date");
        }
        if let Some(t) = &to_date {
            qb.push(" AND o.order_date <= ")
                .push_bind(t.clone())
                .push("::date");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM orders o JOIN customers c \
         ON c.id=o.customer_id AND c.tenant_id=o.tenant_id",
    );
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(ORDER_SELECT);
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by} "));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let mut items = qb.build_query_as::<Order>().fetch_all(pool).await?;
    for o in &mut items {
        o.items = Vec::new();
    }
    Ok(Page::new(items, total, page, page_size))
}

/// Updates an order's status / dates / notes within a transaction.
pub async fn update_order(conn: &mut PgConnection, o: &Order) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE orders SET status=$3, order_date=$4::date, fulfillment_date=$5::date, \
         notes=$6, updated_at=now() WHERE tenant_id=$1 AND id=$2",
    )
    .bind(o.tenant_id)
    .bind(o.id)
    .bind(&o.status)
    .bind(&o.order_date)
    .bind(&o.fulfillment_date)
    .bind(&o.notes)
    .execute(&mut *conn)
    .await
    .map(|_| ())
}

/// Deletes a draft order; returns true if a row was removed.
pub async fn delete_order(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM orders WHERE tenant_id=$1 AND id=$2 AND status='draft'")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- order items ----

/// Inserts an order item; `total_price_pence` is computed by the DB.
pub async fn insert_item(pool: &PgPool, item: &OrderItem) -> Result<OrderItem, sqlx::Error> {
    let sql = format!(
        "INSERT INTO order_items \
         (tenant_id, order_id, batch_id, product_name, volume_liters, unit_price_pence, quantity, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING {ITEM_COLS}"
    );
    sqlx::query_as::<_, OrderItem>(&sql)
        .bind(item.tenant_id)
        .bind(item.order_id)
        .bind(item.batch_id)
        .bind(&item.product_name)
        .bind(item.volume_liters)
        .bind(item.unit_price_pence)
        .bind(item.quantity)
        .bind(&item.notes)
        .fetch_one(pool)
        .await
}

/// Fetches an order item by id, scoped to tenant + order.
pub async fn select_item_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    order_id: Uuid,
    item_id: Uuid,
) -> Result<Option<OrderItem>, sqlx::Error> {
    let sql =
        format!("SELECT {ITEM_COLS} FROM order_items WHERE tenant_id=$1 AND order_id=$2 AND id=$3");
    sqlx::query_as::<_, OrderItem>(&sql)
        .bind(tenant_id)
        .bind(order_id)
        .bind(item_id)
        .fetch_optional(pool)
        .await
}

/// Lists all items for an order, oldest first.
pub async fn select_items_by_order_id(
    pool: &PgPool,
    tenant_id: Uuid,
    order_id: Uuid,
) -> Result<Vec<OrderItem>, sqlx::Error> {
    let sql = format!(
        "SELECT {ITEM_COLS} FROM order_items WHERE tenant_id=$1 AND order_id=$2 ORDER BY created_at"
    );
    sqlx::query_as::<_, OrderItem>(&sql)
        .bind(tenant_id)
        .bind(order_id)
        .fetch_all(pool)
        .await
}

/// Updates an order item's mutable fields; returns the updated row.
pub async fn update_item(
    pool: &PgPool,
    item: &OrderItem,
) -> Result<Option<OrderItem>, sqlx::Error> {
    let sql = format!(
        "UPDATE order_items SET batch_id=$4, product_name=$5, volume_liters=$6, \
         unit_price_pence=$7, quantity=$8, notes=$9 \
         WHERE tenant_id=$1 AND order_id=$2 AND id=$3 RETURNING {ITEM_COLS}"
    );
    sqlx::query_as::<_, OrderItem>(&sql)
        .bind(item.tenant_id)
        .bind(item.order_id)
        .bind(item.id)
        .bind(item.batch_id)
        .bind(&item.product_name)
        .bind(item.volume_liters)
        .bind(item.unit_price_pence)
        .bind(item.quantity)
        .bind(&item.notes)
        .fetch_optional(pool)
        .await
}

/// Deletes an order item; returns true if a row was removed.
pub async fn delete_item(
    pool: &PgPool,
    tenant_id: Uuid,
    order_id: Uuid,
    item_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM order_items WHERE tenant_id=$1 AND order_id=$2 AND id=$3")
        .bind(tenant_id)
        .bind(order_id)
        .bind(item_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- duty events ----

/// Inserts a duty event within a transaction.
pub async fn insert_duty_event(conn: &mut PgConnection, e: &DutyEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO duty_events \
         (tenant_id, order_id, batch_id, event_type, volume_liters, abv_pct, duty_pence, \
         jurisdiction, crystallised_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(e.tenant_id)
    .bind(e.order_id)
    .bind(e.batch_id)
    .bind(&e.event_type)
    .bind(e.volume_liters)
    .bind(e.abv_pct)
    .bind(e.duty_pence)
    .bind(&e.jurisdiction)
    .bind(e.crystallised_at)
    .execute(&mut *conn)
    .await
    .map(|_| ())
}

/// Fetches a duty event by id, tenant-scoped.
pub async fn select_duty_event_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<DutyEvent>, sqlx::Error> {
    let sql = format!("SELECT {DUTY_COLS} FROM duty_events WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, DutyEvent>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists duty events with filters.
pub async fn select_duty_events(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &DutyEventFilter,
    order_by: &str,
) -> Result<Page<DutyEvent>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let from_date = filter.from_date.clone().filter(|s| !s.is_empty());
    let to_date = filter.to_date.clone().filter(|s| !s.is_empty());
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if let Some(o) = filter.order_id {
            qb.push(" AND order_id = ").push_bind(o);
        }
        if let Some(b) = filter.batch_id {
            qb.push(" AND batch_id = ").push_bind(b);
        }
        if let Some(f) = &from_date {
            qb.push(" AND crystallised_at >= ")
                .push_bind(f.clone())
                .push("::timestamptz");
        }
        if let Some(t) = &to_date {
            qb.push(" AND crystallised_at <= ")
                .push_bind(t.clone())
                .push("::timestamptz");
        }
    };

    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM duty_events");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {DUTY_COLS} FROM duty_events"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by} "));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<DutyEvent>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Atomically reads and increments the tenant's `next_order_number`. Must be
/// called inside a transaction (takes a `FOR UPDATE` lock).
pub async fn get_and_increment_order_number(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<i32, sqlx::Error> {
    let n: i32 = sqlx::query_scalar(
        "SELECT COALESCE(next_order_number, 1) FROM tenants WHERE id=$1 FOR UPDATE",
    )
    .bind(tenant_id)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE tenants SET next_order_number = COALESCE(next_order_number, 1) + 1 WHERE id=$1",
    )
    .bind(tenant_id)
    .execute(&mut *conn)
    .await?;
    Ok(n)
}

/// Total revenue (quantity * unit_price_pence) for fulfilled/invoiced items
/// tagged to the given batch.
pub async fn sum_revenue_for_batch(
    pool: &PgPool,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(oi.quantity * oi.unit_price_pence), 0)::bigint \
         FROM order_items oi JOIN orders o ON o.id = oi.order_id \
         WHERE oi.batch_id = $1 AND o.tenant_id = $2 AND o.status IN ('fulfilled', 'invoiced')",
    )
    .bind(batch_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
}
