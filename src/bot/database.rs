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

#[derive(serde::Serialize)]
pub struct FullResponse {
    pub id: Option<i32>,
    pub speech_code: String,
    pub telegram_id: i32,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
}

pub struct User {
    pub telegram_id: i32,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(serde::Serialize)]
pub struct CodeResult {
    pub speech_code: String,
    pub responses: Vec<FullResponse>
}

pub struct UsernameResult {
    pub telegram_id: i32,
    pub username: String,
    pub responses: Vec<FullResponse>
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

    pub async fn flush_responses(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM responses")
            .execute(&self.pool)
            .await.unwrap_or_default();
        Ok(())
    }

    pub async fn flush_codes(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM allowed_codes")
            .execute(&self.pool)
            .await.unwrap_or_default();
        self.flush_responses().await?;
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

    // pub async fn get_all_responses(&self) -> Result<Vec<Response>, sqlx::Error> {
    //     let mut responses: Vec<Response> = Vec::new();
    //     let mut rows = sqlx::query("SELECT * FROM responses")
    //         .fetch(&self.pool);

    //     while let Some(row) = rows.try_next().await? {
    //         responses.push(Response {
    //             id: row.get("id"),
    //             speech_code: row.get("speech_code"),
    //             telegram_id: row.get("telegram_id"),
    //         });
    //     }

    //     Ok(responses)
    // }

    pub async fn vec_response_to_fullresponse(&self, responses: Vec<Response>) -> Result<Vec<FullResponse>, sqlx::Error> {
        let mut full_responses: Vec<FullResponse> = Vec::new();
        for response in responses {
            let user = sqlx::query("SELECT * FROM users WHERE telegram_id = $1")
                .bind(response.telegram_id)
                .fetch_one(&self.pool)
                .await?;
            full_responses.push(FullResponse {
                id: response.id,
                speech_code: response.speech_code,
                telegram_id: response.telegram_id,
                username: user.get("username"),
                first_name: user.get("first_name"),
                last_name: user.get("last_name"),
            });
        }
        Ok(full_responses)
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

    pub async fn get_by_code(&self, code: String) -> Result<Vec<Response>, sqlx::Error> {
        let mut responses: Vec<Response> = Vec::new();
        let mut rows = sqlx::query("SELECT * FROM responses WHERE speech_code = $1")
            .bind(code)
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

    pub async fn get_by_telegram_id(&self, telegram_id: i32) -> Result<Vec<FullResponse>, sqlx::Error> {
        let mut responses: Vec<FullResponse> = Vec::new();
        let mut rows = sqlx::query("SELECT * FROM responses WHERE telegram_id = $1")
            .bind(telegram_id)
            .fetch(&self.pool);

        while let Some(row) = rows.try_next().await? {
            let user = sqlx::query("SELECT * FROM users WHERE telegram_id = $1")
                .bind(telegram_id)
                .fetch_one(&self.pool)
                .await?;
            responses.push(FullResponse {
                id: row.get("id"),
                speech_code: row.get("speech_code"),
                telegram_id: row.get("telegram_id"),
                username: user.get("username"),
                first_name: user.get("first_name"),
                last_name: user.get("last_name"),
            });
        }

        Ok(responses)
    }

    pub async fn get_all_code_results(&self) -> Result<Vec<CodeResult>, sqlx::Error> {
        // Get results from all allowed codes in the database
        let mut code_results: Vec<CodeResult> = Vec::new();
        let codes = self.get_codes().await?;
        for code in codes {
            let responses = self.get_by_code(code.clone()).await?;
            let mut users: Vec<FullResponse> = Vec::new();
            for response in responses {
                let user = sqlx::query("SELECT * FROM users WHERE telegram_id = $1")
                    .bind(response.telegram_id)
                    .fetch_one(&self.pool)
                    .await?;
                users.push(FullResponse {
                    id: response.id,
                    speech_code: response.speech_code,
                    telegram_id: response.telegram_id,
                    username: user.get("username"),
                    first_name: user.get("first_name"),
                    last_name: user.get("last_name"),
                });
            }
            code_results.push(CodeResult {
                speech_code: code,
                responses: users,
            });
        }
        Ok(code_results)
    }

    pub async fn get_all_username_results(&self) -> Result<Vec<UsernameResult>, sqlx::Error> {
        // Get results from all usernames in the database
        let mut username_results: Vec<UsernameResult> = Vec::new();
        let telegram_ids = self.get_users().await?;
        for telegram_id in telegram_ids {
            let responses = self.get_by_telegram_id(telegram_id).await?;
            let username = responses[0].username.clone();
            username_results.push(UsernameResult {
                telegram_id,
                username,
                responses,
            });
        }

        Ok(username_results)
    }

    pub async fn flush_responses_with_unknown_codes(&self) -> Result<(), sqlx::Error> {
        // Strip all responses with codes that are not in the allowed codes list in the database
        let codes = self.get_codes().await?;
        let mut rows = sqlx::query("SELECT * FROM responses")
            .fetch(&self.pool);

        while let Some(row) = rows.try_next().await? {
            let speech_code: String = row.get("speech_code");
            if !codes.contains(&speech_code) {
                sqlx::query("DELETE FROM responses WHERE id = $1")
                    .bind(row.get::<i32, _>("id"))
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
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