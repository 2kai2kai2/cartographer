CREATE TYPE IF NOT EXISTS game_type AS ENUM ('EU4', 'EU5');

CREATE TABLE IF NOT EXISTS games (
    game_id BIGSERIAL PRIMARY KEY,
    server_id BIGINT,
    game_type game_type NOT NULL
);

CREATE TABLE IF NOT EXISTS reservations (
    game_id BIGINT REFERENCES games (game_id) ON DELETE CASCADE,
    user_id BIGINT,
    timestamp TIMESTAMPTZ,
    tag VARCHAR(3),
    PRIMARY KEY (game_id, user_id)
);