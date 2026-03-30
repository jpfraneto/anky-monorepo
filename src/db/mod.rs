pub mod queries;

use anyhow::{anyhow, Result};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Column, PgPool, Row as SqlxRow, ValueRef};
use std::future::Future;

pub type DbPool = PgPool;

pub async fn create_pool(database_url: &str) -> std::result::Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub fn conn(pool: &PgPool) -> Result<Connection> {
    Ok(Connection { pool: pool.clone() })
}

pub fn get_conn_logged(pool: &PgPool) -> Option<Connection> {
    match conn(pool) {
        Ok(conn) => Some(conn),
        Err(e) => {
            tracing::error!("database connection error: {}", e);
            None
        }
    }
}

#[derive(Clone)]
pub struct Connection {
    pool: PgPool,
}

impl Connection {
    pub fn execute<P>(&self, sql: &str, params: P) -> Result<u64>
    where
        P: IntoParams,
    {
        let sql = translate_sql(sql);
        let params = params.into_params();
        block_on_db(async move {
            let query = bind_params(sqlx::query(&sql), params);
            let result = query.execute(&self.pool).await?;
            Ok(result.rows_affected())
        })
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        for statement in split_sql_batch(sql) {
            if statement.trim().is_empty() {
                continue;
            }
            self.execute(&statement, crate::params![])?;
        }
        Ok(())
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement> {
        Ok(Statement {
            pool: self.pool.clone(),
            sql: translate_sql(sql),
        })
    }

    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: IntoParams,
        F: FnOnce(&Row) -> Result<T>,
    {
        let sql = translate_sql(sql);
        let params = params.into_params();
        let row = block_on_db(async move {
            let query = bind_params(sqlx::query(&sql), params);
            query.fetch_one(&self.pool).await
        })?;
        f(&Row { row })
    }
}

pub struct Statement {
    pool: PgPool,
    sql: String,
}

impl Statement {
    pub fn query_row<T, P, F>(&mut self, params: P, f: F) -> Result<T>
    where
        P: IntoParams,
        F: FnOnce(&Row) -> Result<T>,
    {
        let sql = self.sql.clone();
        let params = params.into_params();
        let row = block_on_db(async move {
            let query = bind_params(sqlx::query(&sql), params);
            query.fetch_one(&self.pool).await
        })?;
        f(&Row { row })
    }

    pub fn query_map<T, P, F>(&mut self, params: P, mut f: F) -> Result<MappedRows<T>>
    where
        P: IntoParams,
        F: FnMut(&Row) -> Result<T>,
    {
        let sql = self.sql.clone();
        let params = params.into_params();
        let rows = block_on_db(async move {
            let query = bind_params(sqlx::query(&sql), params);
            query.fetch_all(&self.pool).await
        })?;

        let mapped = rows
            .into_iter()
            .map(|row| f(&Row { row }))
            .collect::<Vec<_>>();

        Ok(MappedRows {
            inner: mapped.into_iter(),
        })
    }
}

pub struct MappedRows<T> {
    inner: std::vec::IntoIter<Result<T>>,
}

impl<T> Iterator for MappedRows<T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct Row {
    row: PgRow,
}

impl Row {
    pub fn get<I, T>(&self, index: I) -> Result<T>
    where
        I: RowIndex,
        T: FromCell,
    {
        let idx = index.resolve(&self.row)?;
        T::from_cell(&self.row, idx)
    }
}

pub trait RowIndex {
    fn resolve(&self, row: &PgRow) -> Result<usize>;
}

impl RowIndex for usize {
    fn resolve(&self, _row: &PgRow) -> Result<usize> {
        Ok(*self)
    }
}

impl RowIndex for &str {
    fn resolve(&self, row: &PgRow) -> Result<usize> {
        row.columns()
            .iter()
            .position(|column| column.name() == *self)
            .ok_or_else(|| anyhow!("column not found: {}", self))
    }
}

pub trait FromCell: Sized {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self>;
}

impl<T> FromCell for Option<T>
where
    T: FromCell,
{
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        let raw = row.try_get_raw(idx)?;
        if raw.is_null() {
            Ok(None)
        } else {
            T::from_cell(row, idx).map(Some)
        }
    }
}

impl FromCell for String {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<String, _>(idx) {
            return Ok(value);
        }
        if let Ok(value) = row.try_get::<i64, _>(idx) {
            return Ok(value.to_string());
        }
        if let Ok(value) = row.try_get::<i32, _>(idx) {
            return Ok(value.to_string());
        }
        if let Ok(value) = row.try_get::<f64, _>(idx) {
            return Ok(value.to_string());
        }
        if let Ok(value) = row.try_get::<bool, _>(idx) {
            return Ok(if value { "1" } else { "0" }.to_string());
        }
        if let Ok(value) = row.try_get::<serde_json::Value, _>(idx) {
            return Ok(value.to_string());
        }
        if let Ok(value) = row.try_get::<chrono::NaiveDateTime, _>(idx) {
            return Ok(value.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(value) = row.try_get::<chrono::NaiveDate, _>(idx) {
            return Ok(value.to_string());
        }
        Err(anyhow!("unsupported string conversion at column {}", idx))
    }
}

impl FromCell for i32 {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<i32, _>(idx) {
            return Ok(value);
        }
        if let Ok(value) = row.try_get::<i64, _>(idx) {
            return Ok(value as i32);
        }
        if let Ok(value) = row.try_get::<bool, _>(idx) {
            return Ok(if value { 1 } else { 0 });
        }
        if let Ok(value) = row.try_get::<String, _>(idx) {
            return value
                .parse::<i32>()
                .map_err(|e| anyhow!("invalid i32 at column {}: {}", idx, e));
        }
        Err(anyhow!("unsupported i32 conversion at column {}", idx))
    }
}

impl FromCell for i64 {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<i64, _>(idx) {
            return Ok(value);
        }
        if let Ok(value) = row.try_get::<i32, _>(idx) {
            return Ok(value as i64);
        }
        if let Ok(value) = row.try_get::<bool, _>(idx) {
            return Ok(if value { 1 } else { 0 });
        }
        if let Ok(value) = row.try_get::<String, _>(idx) {
            return value
                .parse::<i64>()
                .map_err(|e| anyhow!("invalid i64 at column {}: {}", idx, e));
        }
        Err(anyhow!("unsupported i64 conversion at column {}", idx))
    }
}

impl FromCell for u32 {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        Ok(i64::from_cell(row, idx)? as u32)
    }
}

impl FromCell for u64 {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        Ok(i64::from_cell(row, idx)? as u64)
    }
}

impl FromCell for f64 {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<f64, _>(idx) {
            return Ok(value);
        }
        if let Ok(value) = row.try_get::<f32, _>(idx) {
            return Ok(value as f64);
        }
        if let Ok(value) = row.try_get::<i64, _>(idx) {
            return Ok(value as f64);
        }
        if let Ok(value) = row.try_get::<String, _>(idx) {
            return value
                .parse::<f64>()
                .map_err(|e| anyhow!("invalid f64 at column {}: {}", idx, e));
        }
        Err(anyhow!("unsupported f64 conversion at column {}", idx))
    }
}

impl FromCell for bool {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<bool, _>(idx) {
            return Ok(value);
        }
        if let Ok(value) = row.try_get::<i32, _>(idx) {
            return Ok(value != 0);
        }
        if let Ok(value) = row.try_get::<i64, _>(idx) {
            return Ok(value != 0);
        }
        if let Ok(value) = row.try_get::<String, _>(idx) {
            let lowered = value.trim().to_ascii_lowercase();
            return match lowered.as_str() {
                "1" | "true" | "t" | "yes" => Ok(true),
                "0" | "false" | "f" | "no" => Ok(false),
                _ => Err(anyhow!("invalid bool at column {}: {}", idx, value)),
            };
        }
        Err(anyhow!("unsupported bool conversion at column {}", idx))
    }
}

impl FromCell for Vec<u8> {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        row.try_get::<Vec<u8>, _>(idx).map_err(Into::into)
    }
}

impl FromCell for serde_json::Value {
    fn from_cell(row: &PgRow, idx: usize) -> Result<Self> {
        if let Ok(value) = row.try_get::<serde_json::Value, _>(idx) {
            return Ok(value);
        }
        let text = String::from_cell(row, idx)?;
        Ok(serde_json::from_str(&text)?)
    }
}

#[derive(Debug, Clone)]
pub enum Param {
    Null,
    String(String),
    I32(i32),
    I64(i64),
    F64(f64),
    Bytes(Vec<u8>),
}

pub trait ToParam {
    fn to_param(&self) -> Param;
}

pub fn to_param<T>(value: &T) -> Param
where
    T: ToParam + ?Sized,
{
    value.to_param()
}

impl<T> ToParam for Option<T>
where
    T: ToParam,
{
    fn to_param(&self) -> Param {
        match self {
            Some(value) => value.to_param(),
            None => Param::Null,
        }
    }
}

impl<T> ToParam for &T
where
    T: ToParam + ?Sized,
{
    fn to_param(&self) -> Param {
        (*self).to_param()
    }
}

impl ToParam for String {
    fn to_param(&self) -> Param {
        Param::String(self.clone())
    }
}

impl ToParam for str {
    fn to_param(&self) -> Param {
        Param::String(self.to_string())
    }
}

impl ToParam for i32 {
    fn to_param(&self) -> Param {
        Param::I32(*self)
    }
}

impl ToParam for i64 {
    fn to_param(&self) -> Param {
        Param::I64(*self)
    }
}

impl ToParam for u32 {
    fn to_param(&self) -> Param {
        Param::I64(*self as i64)
    }
}

impl ToParam for u64 {
    fn to_param(&self) -> Param {
        Param::I64(*self as i64)
    }
}

impl ToParam for usize {
    fn to_param(&self) -> Param {
        Param::I64(*self as i64)
    }
}

impl ToParam for f32 {
    fn to_param(&self) -> Param {
        Param::F64(*self as f64)
    }
}

impl ToParam for f64 {
    fn to_param(&self) -> Param {
        Param::F64(*self)
    }
}

impl ToParam for bool {
    fn to_param(&self) -> Param {
        Param::I32(if *self { 1 } else { 0 })
    }
}

impl ToParam for Vec<u8> {
    fn to_param(&self) -> Param {
        Param::Bytes(self.clone())
    }
}

impl ToParam for [u8] {
    fn to_param(&self) -> Param {
        Param::Bytes(self.to_vec())
    }
}

impl ToParam for serde_json::Value {
    fn to_param(&self) -> Param {
        Param::String(self.to_string())
    }
}

impl ToParam for uuid::Uuid {
    fn to_param(&self) -> Param {
        Param::String(self.to_string())
    }
}

impl ToParam for chrono::DateTime<chrono::Utc> {
    fn to_param(&self) -> Param {
        Param::String(self.to_rfc3339())
    }
}

impl ToParam for chrono::NaiveDateTime {
    fn to_param(&self) -> Param {
        Param::String(self.format("%Y-%m-%d %H:%M:%S").to_string())
    }
}

pub trait IntoParams {
    fn into_params(self) -> Vec<Param>;
}

impl IntoParams for Vec<Param> {
    fn into_params(self) -> Vec<Param> {
        self
    }
}

impl IntoParams for &[Param] {
    fn into_params(self) -> Vec<Param> {
        self.to_vec()
    }
}

impl<T, const N: usize> IntoParams for [T; N]
where
    T: ToParam,
{
    fn into_params(self) -> Vec<Param> {
        self.into_iter().map(|value| value.to_param()).collect()
    }
}

impl<T> IntoParams for &[T]
where
    T: ToParam,
{
    fn into_params(self) -> Vec<Param> {
        self.iter().map(|value| value.to_param()).collect()
    }
}

#[macro_export]
macro_rules! params {
    () => {
        Vec::<$crate::db::Param>::new()
    };
    ($($value:expr),+ $(,)?) => {{
        let mut params = Vec::<$crate::db::Param>::new();
        $(params.push($crate::db::to_param(&$value));)+
        params
    }};
}

fn bind_params<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    params: Vec<Param>,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    for param in params {
        query = match param {
            Param::Null => query.bind(Option::<String>::None),
            Param::String(value) => query.bind(value),
            Param::I32(value) => query.bind(value),
            Param::I64(value) => query.bind(value),
            Param::F64(value) => query.bind(value),
            Param::Bytes(value) => query.bind(value),
        };
    }
    query
}

fn block_on_db<F, T>(future: F) -> Result<T>
where
    F: Future<Output = std::result::Result<T, sqlx::Error>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future).map_err(Into::into)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(future).map_err(Into::into)
        }
    }
}

fn translate_sql(sql: &str) -> String {
    let trimmed = sql.trim();
    let had_semicolon = trimmed.ends_with(';');
    let mut translated = trimmed.trim_end_matches(';').to_string();
    let is_insert_ignore = translated.contains("INSERT OR IGNORE INTO");

    if is_insert_ignore {
        translated = translated.replacen("INSERT OR IGNORE INTO", "INSERT INTO", 1);
    }

    translated = translated.replace("datetime(", "anky_datetime(");
    translated = translated.replace("date(", "anky_date(");
    translated = translate_placeholders(&translated);

    if is_insert_ignore {
        translated.push_str(" ON CONFLICT DO NOTHING");
    }

    if had_semicolon {
        translated.push(';');
    }

    translated
}

fn translate_placeholders(sql: &str) -> String {
    let chars = sql.as_bytes();
    let mut translated = String::with_capacity(sql.len() + 8);
    let mut index = 1usize;
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == b'?' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }

            translated.push('$');
            if j > i + 1 {
                translated.push_str(&sql[i + 1..j]);
            } else {
                translated.push_str(&index.to_string());
                index += 1;
            }
            i = j;
        } else {
            translated.push(chars[i] as char);
            i += 1;
        }
    }

    translated
}

fn split_sql_batch(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut previous = '\0';

    for ch in sql.chars() {
        match ch {
            '\'' if previous != '\\' => {
                in_single_quote = !in_single_quote;
                current.push(ch);
            }
            ';' if !in_single_quote => {
                if !current.trim().is_empty() {
                    statements.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
        previous = ch;
    }

    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}
