use anyhow::Result;
use rusqlite::{params, Connection};

const EMBEDDING_DIM: usize = 768;

/// Call nomic-embed-text via Ollama to embed a text string.
/// Returns a 768-dimensional f32 vector.
pub async fn embed_text(ollama_base_url: &str, text: &str) -> Result<Vec<f32>> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "nomic-embed-text",
        "input": text,
    });

    let resp = client
        .post(format!("{}/api/embed", ollama_base_url))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama embed API error {}: {}", status, body);
    }

    let data: serde_json::Value = resp.json().await?;
    let embedding = data["embeddings"][0]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing embeddings in response"))?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect::<Vec<f32>>();

    if embedding.len() != EMBEDDING_DIM {
        anyhow::bail!(
            "unexpected embedding dimension: {} (expected {})",
            embedding.len(),
            EMBEDDING_DIM
        );
    }

    Ok(embedding)
}

/// Serialize an f32 vector into bytes for storage in SQLite BLOB.
pub fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// Deserialize bytes back into an f32 vector.
pub fn bytes_to_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Store an embedding for a writing session in the database.
pub fn store_embedding(
    conn: &Connection,
    id: &str,
    user_id: &str,
    writing_session_id: Option<&str>,
    source: &str,
    content: &str,
    embedding: &[f32],
) -> Result<()> {
    let blob = vec_to_bytes(embedding);
    conn.execute(
        "INSERT OR REPLACE INTO memory_embeddings (id, user_id, writing_session_id, source, content, embedding)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, user_id, writing_session_id, source, content, blob],
    )?;
    Ok(())
}

/// Search for the top N most similar embeddings for a user.
/// Returns (id, writing_session_id, source, content, similarity_score).
pub fn search_similar(
    conn: &Connection,
    user_id: &str,
    query_embedding: &[f32],
    limit: usize,
    min_score: f32,
) -> Result<Vec<(String, Option<String>, String, String, f32)>> {
    let mut stmt = conn.prepare(
        "SELECT id, writing_session_id, source, content, embedding
         FROM memory_embeddings
         WHERE user_id = ?1",
    )?;

    let rows = stmt.query_map(params![user_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Vec<u8>>(4)?,
        ))
    })?;

    let mut results: Vec<(String, Option<String>, String, String, f32)> = Vec::new();
    for row in rows {
        let (id, session_id, source, content, blob) = row?;
        // Skip embeddings with wrong dimensions (legacy OpenAI 1536-dim vectors)
        if blob.len() != EMBEDDING_DIM * 4 {
            continue;
        }
        let stored = bytes_to_vec(&blob);
        let score = cosine_similarity(query_embedding, &stored);
        if score >= min_score {
            results.push((id, session_id, source, content, score));
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);

    Ok(results)
}

/// Count how many embeddings exist for a user.
pub fn count_user_embeddings(conn: &Connection, user_id: &str) -> Result<i32> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM memory_embeddings WHERE user_id = ?1",
        params![user_id],
        |row| row.get(0),
    )?;
    Ok(count)
}
