use omoikane_control::OmoikaneLaunchConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, Pool, Postgres};

#[derive(Clone)]
pub struct OmoikaneSqlState {
    pool: OmoikaneSqlPool,
    redacted_url: String,
    max_connections: u32,
}

#[derive(Clone)]
enum OmoikaneSqlPool {
    Postgres(Pool<Postgres>),
}

impl OmoikaneSqlState {
    pub async fn connect_from_config(
        config: &OmoikaneLaunchConfig,
    ) -> Result<Option<Self>, sqlx::Error> {
        let Some(database_url) = config.database_url.as_deref() else {
            return Ok(None);
        };

        let pool = if database_url.starts_with("postgres://")
            || database_url.starts_with("postgresql://")
        {
            OmoikaneSqlPool::Postgres(
                PgPoolOptions::new()
                    .max_connections(config.database_max_connections)
                    .connect(database_url)
                    .await?,
            )
        } else {
            return Err(sqlx::Error::Configuration(
                "unsupported Omoikane database URL; expected postgres:// or postgresql://".into(),
            ));
        };

        Ok(Some(Self {
            pool,
            redacted_url: redact_database_url(database_url),
            max_connections: config.database_max_connections,
        }))
    }

    pub async fn check(&self) -> Result<(), sqlx::Error> {
        match &self.pool {
            OmoikaneSqlPool::Postgres(pool) => pool.execute("SELECT 1").await.map(|_| ()),
        }
    }

    pub fn redacted_url(&self) -> &str {
        &self.redacted_url
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }
}

fn redact_database_url(database_url: &str) -> String {
    let Some(scheme_end) = database_url.find("://") else {
        return "<redacted-sql-url>".to_string();
    };
    let after_scheme = scheme_end + 3;
    let Some(at_index) = database_url[after_scheme..].find('@') else {
        return database_url.to_string();
    };
    let at_index = after_scheme + at_index;
    let authority = &database_url[after_scheme..at_index];
    let redacted_authority = authority
        .split_once(':')
        .map(|(user, _)| format!("{user}:***"))
        .unwrap_or_else(|| "***".to_string());
    format!(
        "{}{}{}",
        &database_url[..after_scheme],
        redacted_authority,
        &database_url[at_index..]
    )
}

#[cfg(test)]
mod tests {
    use super::redact_database_url;

    #[test]
    fn database_urls_are_redacted_for_terminal_and_json() {
        assert_eq!(
            redact_database_url("postgres://omoikane:secret@db.local/game"),
            "postgres://omoikane:***@db.local/game"
        );
        assert_eq!(
            redact_database_url("mysql://db.local/game"),
            "mysql://db.local/game"
        );
    }
}
