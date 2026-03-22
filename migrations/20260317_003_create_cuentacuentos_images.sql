CREATE TABLE IF NOT EXISTS cuentacuentos_images (
    id TEXT PRIMARY KEY,
    cuentacuentos_id TEXT NOT NULL,
    phase_index INTEGER NOT NULL,
    image_prompt TEXT NOT NULL,
    image_url TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    generated_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (cuentacuentos_id, phase_index),
    FOREIGN KEY (cuentacuentos_id) REFERENCES cuentacuentos(id)
);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_images_pending
    ON cuentacuentos_images(cuentacuentos_id, status, phase_index);

CREATE INDEX IF NOT EXISTS idx_cuentacuentos_images_story
    ON cuentacuentos_images(cuentacuentos_id, phase_index);
