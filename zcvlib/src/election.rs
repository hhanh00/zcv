use sqlx::{SqliteConnection, query, query_as};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, QuestionPropHashable},
};

impl ElectionPropsPub {
    pub async fn store(&self, conn: &mut SqliteConnection) -> ZCVResult<()> {
        let hash = self.hash()?;
        let (election,): (u32,) = query_as(
            "INSERT INTO elections
            (hash, start, end, need_sig, name)
            VALUES (?, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET
            start = excluded.start,
            end = excluded.end,
            need_sig = excluded.need_sig,
            name = excluded.name
            RETURNING id_election",
        )
        .bind(hash.as_slice())
        .bind(self.start)
        .bind(self.end)
        .bind(self.need_sig)
        .bind(&self.name)
        .fetch_one(&mut *conn)
        .await?;
        for (i, q) in self.questions.iter().enumerate() {
            let q_js = serde_json::to_string(q).anyhow()?;
            let domain = QuestionPropHashable::for_question(self, i).calculate_domain()?;
            query(
                "INSERT INTO questions
                (election, idx, domain, title, subtitle, data)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT DO UPDATE SET
                domain = excluded.domain,
                title = excluded.title,
                subtitle = excluded.subtitle,
                data = excluded.data",
            )
            .bind(election)
            .bind(i as u32)
            .bind(domain.as_slice())
            .bind(&q.title)
            .bind(&q.subtitle)
            .bind(q_js)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;
    use sqlx::{Sqlite, pool::PoolConnection};

    use crate::{context::Context, db::create_schema, pod::ElectionProps};

    async fn setup() -> Result<PoolConnection<Sqlite>> {
        let ctx = Context::new("vote.db", "").await?;
        let mut conn = ctx.connect().await?;
        create_schema(&mut conn).await?;
        Ok(conn)
    }

    #[tokio::test]
    async fn test_store_election() -> Result<()> {
        let mut conn = setup().await?;
        let e = json!({
            "secret_seed": "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew",
            "start": 3155000,
            "end": 3169000,
            "need_sig": true,
            "name": "Test Election",
            "questions": [
                {
                    "title": "Q1. What is your favorite color?",
                    "subtitle": "",
                    "answers": ["Red", "Green", "Blue"]
                },
                {
                    "title": "Q2. Is the earth flat?",
                    "subtitle": "",
                    "answers": ["Yes", "No"]
                },
                {
                    "title": "Q3. Do you like pizza?",
                    "subtitle": "",
                    "answers": ["Yes", "No"]
                },
            ]
        });
        let e: ElectionProps = serde_json::from_value(e).unwrap();
        let e = e.build()?;
        e.store(&mut conn).await?;
        Ok(())
    }
}
