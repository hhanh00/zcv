use sqlx::{SqliteConnection, query, query_as};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, QuestionPropHashable},
};

impl ElectionPropsPub {
    pub async fn store(&self, conn: &mut SqliteConnection) -> ZCVResult<()> {
        let hash = self.hash()?;
        let json = serde_json::to_string(self).anyhow()?;
        let (election,): (u32,) = query_as(
            "INSERT INTO elections
            (hash, start, end, need_sig, name, data)
            VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET
            start = excluded.start,
            end = excluded.end,
            need_sig = excluded.need_sig,
            name = excluded.name,
            data = excluded.data
            RETURNING id_election",
        )
        .bind(hash.as_slice())
        .bind(self.start)
        .bind(self.end)
        .bind(self.need_sig)
        .bind(&self.name)
        .bind(&json)
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
