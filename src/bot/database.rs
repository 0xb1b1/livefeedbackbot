//use std::sync::Arc;

use sqlx::postgres::{PgPoolOptions,
    //PgRow
};
//use sqlx::{FromRow, Row};
use futures::TryStreamExt;
use sqlx::Row;  // import for get() function on sqlx queries

pub struct Response {
    pub id: Option<i32>,
    pub speech_code: String,
    pub telegram_id: i32,
}

pub struct User {
    pub telegram_id: i32,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
}

pub struct Database {
    pub pool: sqlx::PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query("CREATE TABLE IF NOT EXISTS responses (
            id SERIAL PRIMARY KEY,
            speech_code VARCHAR(32) NOT NULL,
            telegram_id INT NOT NULL,
            UNIQUE (telegram_id, speech_code)
        )")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS allowed_codes (
            id SERIAL PRIMARY KEY,
            code VARCHAR(32) NOT NULL,
            UNIQUE (code)
        )")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS users (
            telegram_id INT PRIMARY KEY,
            first_name VARCHAR(64) NOT NULL,
            last_name VARCHAR(64) NOT NULL,
            username VARCHAR(32) NOT NULL,
            UNIQUE (telegram_id)
        )")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_user(&self, user: &User) -> Result<(), sqlx::Error> {
        // Check if user already exists
        if sqlx::query("SELECT telegram_id FROM users WHERE telegram_id = $1")
            .bind(&user.telegram_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some() {
            return Ok(());
        }
        sqlx::query("INSERT INTO users (telegram_id, username, first_name, last_name) VALUES ($1, $2, $3, $4)")
            .bind(&user.telegram_id)
            .bind(&user.username)
            .bind(&user.first_name)
            .bind(&user.last_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_users(&self) -> Result<Vec<i32>, sqlx::Error> {
        let mut users: Vec<i32> = Vec::new();
        let mut rows = sqlx::query("SELECT telegram_id FROM users")
            .fetch(&self.pool);
        while let Some(row) = rows.try_next().await? {
            users.push(row.get("telegram_id"));
        }
        Ok(users)
    }

    pub async fn add_code(&self, code: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO allowed_codes (code) VALUES ($1)")
            .bind(code)
            .execute(&self.pool)
            .await.unwrap_or_default();  // See how to replace Result return statement to fit this unwrap
        Ok(())
    }

    pub async fn del_code(&self, code: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM allowed_codes WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await.unwrap_or_default();
        Ok(())
    }

    pub async fn flush_codes(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM allowed_codes")
            .execute(&self.pool)
            .await.unwrap_or_default();
        sqlx::query("DELETE FROM responses")
            .execute(&self.pool)
            .await.unwrap_or_default();
        Ok(())
    }

    pub async fn get_codes(&self) -> Result<Vec<String>, sqlx::Error> {
        let mut codes: Vec<String> = Vec::new();
        let mut rows = sqlx::query("SELECT * FROM allowed_codes")
            .fetch(&self.pool);
        while let Some(row) = rows.try_next().await? {
            codes.push(row.get("code"));
        }
        Ok(codes)
    }

    pub async fn is_code_allowed(&self, code: &str) -> Result<bool, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM allowed_codes WHERE code = $1")
            .bind(code)
            .fetch_one(&self.pool)
            .await;
        Ok(row.is_ok())
    }

    pub async fn insert(&self, response: Response) -> Result<(), sqlx::Error> {
        // Executes query; fails silently if unique match already exists
        sqlx::query("INSERT INTO responses (speech_code, telegram_id) VALUES ($1, $2)")
            .bind(response.speech_code)
            .bind(response.telegram_id)
            .execute(&self.pool)
            .await.unwrap_or_default();
        Ok(())
    }

    pub async fn get_users_by_code(&self, code: String) -> Result<Vec<i32>, sqlx::Error> {
        let mut users: Vec<i32> = Vec::new();
        let mut rows = sqlx::query("SELECT telegram_id FROM responses WHERE speech_code = $1")
            .bind(code)
            .fetch(&self.pool);

        while let Some(row) = rows.try_next().await? {
            users.push(row.get("telegram_id"));
        }

        Ok(users)
    }

    pub async fn get_by_user(&self, user_id: i32) -> Result<Vec<Response>, sqlx::Error> {
        let mut responses: Vec<Response> = Vec::new();
        let mut rows = sqlx::query("SELECT * FROM responses WHERE telegram_id = $1")
            .bind(user_id)
            .fetch(&self.pool);

        while let Some(row) = rows.try_next().await? {
            responses.push(Response {
                id: Some(row.get(0)),
                speech_code: row.get(1),
                telegram_id: row.get(2),
            });
        }

        Ok(responses)
    }

    // pub async fn get_all(&self) -> Result<Vec<Response>, sqlx::Error> {
    //     let mut responses: Vec<Response> = Vec::new();
    //     let mut rows = sqlx::query("SELECT * FROM responses")
    //         .fetch(&self.pool);

    //     while let Some(row) = rows.try_next().await {
    //     }

    //     Ok(vec![])
    // }
}