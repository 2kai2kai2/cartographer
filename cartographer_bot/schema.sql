CREATE TABLE IF NOT EXISTS games (
    game_id BIGSERIAL PRIMARY KEY,
    server_id BIGINT
);

CREATE TABLE IF NOT EXISTS reservations (
    game_id BIGINT REFERENCES games (game_id) ON DELETE CASCADE,
    user_id BIGINT,
    timestamp TIMESTAMPTZ,
    tag VARCHAR(3),
    PRIMARY KEY (game_id, user_id)
);