//! Data access for suppliers, purchase orders, and lines.
//!
//! Port of the Go `internal/procurement/repository.go`. `order_date` and
//! `expected_delivery` are `DATE` columns rendered with `to_char` and bound back
//! via `::date`. `quantity` and `received_quantity` are `NUMERIC` (selected as
//! `float8`). Every supplier/PO query is tenant-scoped.

use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::models::{POFilter, Page, PurchaseOrder, PurchaseOrderLine, Supplier, SupplierFilter};

const SUPPLIER_COLS: &str = "id, tenant_id, name, contact_name, email, phone, website, notes, \
    created_at, updated_at";

const PO_COLS: &str =
    "po.id, po.tenant_id, po.supplier_id, s.name AS supplier_name, po.po_number, \
    po.status, to_char(po.order_date, 'YYYY-MM-DD') AS order_date, \
    to_char(po.expected_delivery, 'YYYY-MM-DD') AS expected_delivery, po.notes, \
    po.created_at, po.updated_at";

const PO_FROM: &str = "FROM purchase_orders po JOIN suppliers s ON s.id = po.supplier_id";

const LINE_COLS: &str = "id, purchase_order_id, ingredient_type, ingredient_name, \
    quantity::float8 AS quantity, unit, unit_cost_pence, unit_cost_currency, \
    received_quantity::float8 AS received_quantity, created_at, updated_at";

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

fn supplier_order_by(sort: &str) -> &'static str {
    match sort {
        "name" => "name ASC",
        "-name" => "name DESC",
        "created_at" => "created_at ASC",
        "-created_at" => "created_at DESC",
        _ => "name ASC",
    }
}

fn po_order_by(sort: &str) -> &'static str {
    match sort {
        "order_date" => "po.order_date ASC",
        "-order_date" => "po.order_date DESC",
        "created_at" => "po.created_at ASC",
        "-created_at" => "po.created_at DESC",
        _ => "po.created_at DESC",
    }
}

// ---- suppliers ----

/// Inserts a supplier and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_supplier(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    contact_name: Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
    website: Option<&str>,
    notes: Option<&str>,
) -> Result<Supplier, sqlx::Error> {
    let sql = format!(
        "INSERT INTO suppliers (tenant_id, name, contact_name, email, phone, website, notes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {SUPPLIER_COLS}"
    );
    sqlx::query_as::<_, Supplier>(&sql)
        .bind(tenant_id)
        .bind(name)
        .bind(contact_name)
        .bind(email)
        .bind(phone)
        .bind(website)
        .bind(notes)
        .fetch_one(pool)
        .await
}

/// Fetches a supplier by id, tenant-scoped.
pub async fn select_supplier_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<Supplier>, sqlx::Error> {
    let sql = format!("SELECT {SUPPLIER_COLS} FROM suppliers WHERE tenant_id=$1 AND id=$2");
    sqlx::query_as::<_, Supplier>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Lists suppliers with an optional name-prefix search.
pub async fn select_suppliers(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &SupplierFilter,
) -> Result<Page<Supplier>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let search = filter.search.clone();
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
        if !search.is_empty() {
            qb.push(" AND name ILIKE ")
                .push_bind(search.clone())
                .push(" || '%'");
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM suppliers");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = supplier_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {SUPPLIER_COLS} FROM suppliers"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let items = qb.build_query_as::<Supplier>().fetch_all(pool).await?;
    Ok(Page::new(items, total, page, page_size))
}

/// Updates a supplier's mutable fields and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn update_supplier(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    name: &str,
    contact_name: Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
    website: Option<&str>,
    notes: Option<&str>,
) -> Result<Option<Supplier>, sqlx::Error> {
    let sql = format!(
        "UPDATE suppliers SET name=$3, contact_name=$4, email=$5, phone=$6, website=$7, \
         notes=$8, updated_at=now() WHERE tenant_id=$1 AND id=$2 RETURNING {SUPPLIER_COLS}"
    );
    sqlx::query_as::<_, Supplier>(&sql)
        .bind(tenant_id)
        .bind(id)
        .bind(name)
        .bind(contact_name)
        .bind(email)
        .bind(phone)
        .bind(website)
        .bind(notes)
        .fetch_optional(pool)
        .await
}

/// Deletes a supplier; returns true if a row was removed.
pub async fn delete_supplier(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM suppliers WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// True if the supplier has any purchase orders.
pub async fn supplier_has_pos(
    pool: &PgPool,
    tenant_id: Uuid,
    supplier_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM purchase_orders WHERE tenant_id=$1 AND supplier_id=$2",
    )
    .bind(tenant_id)
    .bind(supplier_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

// ---- purchase orders ----

/// Atomically reads and increments the tenant's `next_po_number`. Must be called
/// inside a transaction (takes a `FOR UPDATE` lock).
pub async fn get_and_increment_po_number(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<i32, sqlx::Error> {
    let n: i32 = sqlx::query_scalar(
        "SELECT COALESCE(next_po_number, 1) FROM tenants WHERE id=$1 FOR UPDATE",
    )
    .bind(tenant_id)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("UPDATE tenants SET next_po_number = COALESCE(next_po_number, 1) + 1 WHERE id=$1")
        .bind(tenant_id)
        .execute(&mut *conn)
        .await?;
    Ok(n)
}

/// Inserts a purchase order (order_date = CURRENT_DATE) and returns it. Must be
/// called inside the same transaction as `get_and_increment_po_number`.
#[allow(clippy::too_many_arguments)]
pub async fn insert_po(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    supplier_id: Uuid,
    supplier_name: &str,
    po_number: &str,
    status: &str,
    expected_delivery: Option<&str>,
    notes: Option<&str>,
) -> Result<PurchaseOrder, sqlx::Error> {
    // `supplier_name` is not a column on purchase_orders; bind it as a literal so
    // the RETURNING row satisfies `PurchaseOrder`'s FromRow.
    let sql = "INSERT INTO purchase_orders \
        (tenant_id, supplier_id, po_number, status, order_date, expected_delivery, notes) \
        VALUES ($1,$2,$3,$4,CURRENT_DATE,$5::date,$6) \
        RETURNING id, tenant_id, supplier_id, $7::text AS supplier_name, po_number, status, \
            to_char(order_date, 'YYYY-MM-DD') AS order_date, \
            to_char(expected_delivery, 'YYYY-MM-DD') AS expected_delivery, notes, \
            created_at, updated_at";
    let mut po = sqlx::query_as::<_, PurchaseOrder>(sql)
        .bind(tenant_id)
        .bind(supplier_id)
        .bind(po_number)
        .bind(status)
        .bind(expected_delivery)
        .bind(notes)
        .bind(supplier_name)
        .fetch_one(&mut *conn)
        .await?;
    po.lines = Vec::new();
    Ok(po)
}

/// Fetches a purchase order by id (with supplier_name and its lines), tenant-scoped.
pub async fn select_po_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Option<PurchaseOrder>, sqlx::Error> {
    let sql = format!("SELECT {PO_COLS} {PO_FROM} WHERE po.tenant_id=$1 AND po.id=$2");
    let po = sqlx::query_as::<_, PurchaseOrder>(&sql)
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match po {
        Some(mut po) => {
            po.lines = select_lines_by_po(pool, id).await?;
            Ok(Some(po))
        }
        None => Ok(None),
    }
}

/// Lists purchase orders with filters. List items carry empty `lines`.
pub async fn select_pos(
    pool: &PgPool,
    tenant_id: Uuid,
    filter: &POFilter,
) -> Result<Page<PurchaseOrder>, sqlx::Error> {
    let (page, page_size) = clamp_page(filter.page, filter.page_size);
    let push_where = |qb: &mut QueryBuilder<Postgres>| {
        qb.push(" WHERE po.tenant_id = ").push_bind(tenant_id);
        if let Some(s) = filter.supplier_id {
            qb.push(" AND po.supplier_id = ").push_bind(s);
        }
        if let Some(st) = &filter.status {
            qb.push(" AND po.status = ").push_bind(st.clone());
        }
    };
    let mut count_qb = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM purchase_orders po");
    push_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let order_by = po_order_by(&filter.sort);
    let mut qb = QueryBuilder::<Postgres>::new(format!("SELECT {PO_COLS} {PO_FROM}"));
    push_where(&mut qb);
    qb.push(format!(" ORDER BY {order_by}"));
    qb.push(" LIMIT ").push_bind(page_size);
    qb.push(" OFFSET ").push_bind((page - 1) * page_size);
    let mut items = qb.build_query_as::<PurchaseOrder>().fetch_all(pool).await?;
    for po in &mut items {
        po.lines = Vec::new();
    }
    Ok(Page::new(items, total, page, page_size))
}

/// Updates the mutable PO fields (status, expected_delivery, notes).
pub async fn update_po(
    pool: &PgPool,
    tenant_id: Uuid,
    id: Uuid,
    status: &str,
    expected_delivery: Option<&str>,
    notes: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE purchase_orders SET status=$3, expected_delivery=$4::date, notes=$5, \
         updated_at=now() WHERE tenant_id=$1 AND id=$2",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(status)
    .bind(expected_delivery)
    .bind(notes)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a purchase order; returns true if a row was removed.
pub async fn delete_po(pool: &PgPool, tenant_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM purchase_orders WHERE tenant_id=$1 AND id=$2")
        .bind(tenant_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---- lines ----

/// Inserts a purchase-order line and returns it.
#[allow(clippy::too_many_arguments)]
pub async fn insert_line(
    pool: &PgPool,
    purchase_order_id: Uuid,
    ingredient_type: &str,
    ingredient_name: &str,
    quantity: f64,
    unit: &str,
    unit_cost_pence: i64,
    unit_cost_currency: &str,
) -> Result<PurchaseOrderLine, sqlx::Error> {
    let sql = "INSERT INTO purchase_order_lines \
        (purchase_order_id, ingredient_type, ingredient_name, quantity, unit, unit_cost_pence, \
         unit_cost_currency) \
        VALUES ($1,$2,$3,$4,$5,$6,$7) \
        RETURNING id, purchase_order_id, ingredient_type, ingredient_name, \
            quantity::float8 AS quantity, unit, unit_cost_pence, unit_cost_currency, \
            received_quantity::float8 AS received_quantity, created_at, updated_at";
    sqlx::query_as::<_, PurchaseOrderLine>(sql)
        .bind(purchase_order_id)
        .bind(ingredient_type)
        .bind(ingredient_name)
        .bind(quantity)
        .bind(unit)
        .bind(unit_cost_pence)
        .bind(unit_cost_currency)
        .fetch_one(pool)
        .await
}

/// Fetches a line by (purchase_order_id, line id).
pub async fn select_line_by_id(
    pool: &PgPool,
    po_id: Uuid,
    line_id: Uuid,
) -> Result<Option<PurchaseOrderLine>, sqlx::Error> {
    let sql = format!(
        "SELECT {LINE_COLS} FROM purchase_order_lines WHERE purchase_order_id=$1 AND id=$2"
    );
    sqlx::query_as::<_, PurchaseOrderLine>(&sql)
        .bind(po_id)
        .bind(line_id)
        .fetch_optional(pool)
        .await
}

/// Lists a purchase order's lines (ordered by created_at ASC).
pub async fn select_lines_by_po(
    pool: &PgPool,
    po_id: Uuid,
) -> Result<Vec<PurchaseOrderLine>, sqlx::Error> {
    let sql = format!(
        "SELECT {LINE_COLS} FROM purchase_order_lines WHERE purchase_order_id=$1 \
         ORDER BY created_at ASC"
    );
    sqlx::query_as::<_, PurchaseOrderLine>(&sql)
        .bind(po_id)
        .fetch_all(pool)
        .await
}

/// Updates a line's mutable fields.
#[allow(clippy::too_many_arguments)]
pub async fn update_line(
    pool: &PgPool,
    line_id: Uuid,
    ingredient_type: &str,
    ingredient_name: &str,
    quantity: f64,
    unit: &str,
    unit_cost_pence: i64,
    unit_cost_currency: &str,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE purchase_order_lines SET ingredient_type=$2, ingredient_name=$3, quantity=$4, \
         unit=$5, unit_cost_pence=$6, unit_cost_currency=$7, updated_at=now() WHERE id=$1",
    )
    .bind(line_id)
    .bind(ingredient_type)
    .bind(ingredient_name)
    .bind(quantity)
    .bind(unit)
    .bind(unit_cost_pence)
    .bind(unit_cost_currency)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Deletes a line by id.
pub async fn delete_line(pool: &PgPool, line_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM purchase_order_lines WHERE id=$1")
        .bind(line_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Updates a line's received quantity.
pub async fn update_line_received_qty(
    pool: &PgPool,
    line_id: Uuid,
    qty: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE purchase_order_lines SET received_quantity=$2, updated_at=now() WHERE id=$1",
    )
    .bind(line_id)
    .bind(qty)
    .execute(pool)
    .await?;
    Ok(())
}
